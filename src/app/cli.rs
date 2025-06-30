use crate::common::{Result, RouterError};
use crate::router::SessionCreationReturnCodes;
use crate::fs::chmod;

use base64::engine::{general_purpose::STANDARD, Engine};
use rand::{
    rng, 
    Rng,
    distr::Alphanumeric,
};
use std::{
    fs::File,
    io::Write,
    thread,
    time,
    sync::{Mutex, Arc},
};
use std::time::{Duration, Instant};

struct CommResponse {
    pub _publisher_port: u32,
    pub _subscriber_port: u32,
    pub session_port: u32,
    pub public_key: String,
}

pub struct CreationResponse {
    pub code: SessionCreationReturnCodes,
    pub message: String,
}

struct SessionSocket {
    pub port: u32,
    pub socket: zmq::Socket,
}

pub struct Cli {
    session_socket: Option<SessionSocket>,
}

impl Cli {
    pub fn new() -> Self {
        Self {
            session_socket: None,
        }
    }

    pub fn connect(&mut self, connector_port: u32) -> Result<()> {
        debug!("Connecting to WebX Router");

        // Create ZMQ context
        let context = zmq::Context::new();

        debug!("Connecting to connector socket on port {}", connector_port);
        let connector_socket = self.create_req_socket(&context, connector_port, None)?;

        debug!("Sending comm request...");
        let response = self.send(&connector_socket, "comm")?;
        let comm_response = self.decode_comm_response(&response)?;
        debug!("... received comm response {}", &response);

        debug!("Got session socket port {}", comm_response.session_port);

        let session_socket = self.create_req_socket(&context, comm_response.session_port, Some(comm_response.public_key))?;
        let _ = self.session_socket.insert(SessionSocket { port: comm_response.session_port, socket: session_socket });

        self.disconnect_req_socket(&connector_socket, connector_port);

        info!("Connected to WebX Router");

        Ok(())
    }

    pub fn disconnect(&mut self) {
        if let Some(session_socket) = self.session_socket.as_ref() {
            self.disconnect_req_socket(&session_socket.socket, session_socket.port);
            debug!("Disconnected from WebX Router");
        }
    }

    pub fn create(&self, width: u32, height: u32, keyboard_layout: &str) -> Result<CreationResponse> {
        let session_socket = self.session_socket.as_ref().ok_or_else(|| RouterError::SystemError(format!("Session Socket is unavailable")))?;

        info!("Creating WebX Engine Session command with resolution {} x {} and keyboard layout {}", width, height, keyboard_layout);

        // Create credentials file in /tmp (ro by the user)
        let credentials_path = format!("/tmp/{}", self.create_random_string(8));
        let mut file = File::create(&credentials_path)?;
        chmod(&credentials_path, 0o600)?;

        // create and write a random password
        let password = self.create_random_string(32);
        file.write_all(password.as_bytes())?;

        debug!("Credentials written to {}", credentials_path);

        debug!("Sending creation request to WebX Router...");
        let create_request = format!("create,{},{},{},{},{}", self.encode_base64(&credentials_path), self.encode_base64(&password), width, height, keyboard_layout);
        let response = self.send(&session_socket.socket, &create_request)?;

        debug!("... received response {}", response);

        std::fs::remove_file(&credentials_path)?;

        self.decode_create_response(&response)
    }

    pub fn wait_for_interrupt(&self, session_id: &str) -> Result<()> {
        let session_socket = self.session_socket.as_ref().ok_or_else(|| RouterError::SystemError(format!("Session Socket is unavailable")))?;

        let running_mutex = Arc::new(Mutex::new(true));
        let mut is_running = true;

        let running_handler = Arc::clone(&running_mutex);
        ctrlc::set_handler(move || {
            debug!("CTRL-C received");
            if let Ok(mut running) = running_handler.lock() {
               *running = false;
            }

        }).expect("Error setting Ctrl-C handler");

        let mut last_ping = Instant::now();
        let ping_request = format!("ping,{}", session_id);
        while is_running {
            thread::sleep(time::Duration::from_millis(100));

            if last_ping.elapsed() >= Duration::from_secs(5) {
                debug!("Sending ping request to WebX Router...");
                let ping_response = self.send(&session_socket.socket, &ping_request)?;
                debug!("... received response {}", ping_response);
                last_ping = Instant::now();

                if !self.decode_ping_response(&ping_response) {
                    return Err(RouterError::EngineSessionError(format!("Failed to ping engine")));
                }
            }

            if let Ok(running) = running_mutex.lock() {
                is_running = *running;
            }
        }

        info!("Finished nicely");

        Ok(())
    }

    fn decode_comm_response(&self, response: &str) -> Result<CommResponse> {
        let response_parts = response.split(',').collect::<Vec<&str>>();

        if response_parts.len() < 4 {
            return Err(RouterError::TransportError(format!("Received invalid response from client connector")));
        }

        let _publisher_port: u32 = response_parts[0].parse()?;
        let _subscriber_port: u32 = response_parts[1].parse()?;
        let session_port: u32 = response_parts[2].parse()?;
        let public_key: String = response_parts[3].to_string();
        let comm_response = CommResponse {
            _publisher_port,
            _subscriber_port,
            session_port,
            public_key,
        };

        Ok(comm_response)
    }

    fn decode_create_response(&self, response: &str) -> Result<CreationResponse> {
        let response_parts = response.split(',').collect::<Vec<&str>>();
        let response_code_num: u32 = response_parts[0].parse()?;
        let message = response_parts[1].to_string();
        let code = SessionCreationReturnCodes::try_from(response_code_num)?;

        Ok( CreationResponse { code, message })
    }

    fn decode_ping_response(&self, response: &str) -> bool {
        let response_parts = response.split(',').collect::<Vec<&str>>();

        response_parts[0] == "pong"
    }

    fn send(&self, socket: &zmq::Socket, request: &str) -> Result<String> {
        socket.send(request, 0)?;
        let mut message = zmq::Message::new();
        socket.recv(&mut message, 0)?;

        let text: String = message.as_str().unwrap().to_string();

        Ok(text)
    }

    fn create_req_socket(&self, context: &zmq::Context, port: u32, public_key_option: Option<String>) -> Result<zmq::Socket> {
        let socket = context.socket(zmq::REQ)?;
        socket.set_linger(0)?;
        socket.set_rcvtimeo(5000)?;

        if let Some(public_key_string) = public_key_option {
            // Secure the socket 
            let server_key = zmq::z85_decode(&public_key_string)?;
            socket.set_curve_serverkey(&server_key)?;

            let key_pair = zmq::CurveKeyPair::new()?;
            socket.set_curve_publickey(&key_pair.public_key)?;
            socket.set_curve_secretkey(&key_pair.secret_key)?;
        }

        let address = format!("tcp://localhost:{}", port);
        if let Err(error) = socket.connect(address.as_str()) {
            return Err(RouterError::TransportError(format!("Failed to connect REQ socket to {}: {}", address, error)));
        }

        Ok(socket)
    }

    fn disconnect_req_socket(&self, socket: &zmq::Socket, port: u32) {
        let address = format!("tcp://localhost:{}", port);
        if let Err(error) = socket.disconnect(&address) {
            warn!("Failed to disconnect from Engine Connector socket at {}: {}", address, error)
        }
    }

    fn encode_base64(&self, input: &str) -> String {
        STANDARD.encode(input)
    }

    fn create_random_string(&self, length: usize) -> String {
        rng()
            .sample_iter(&Alphanumeric)
            .take(length)
            .map(char::from)
            .collect()
    }
}

