use crate::common::{Settings, Result, RouterError};
use crate::router::SessionCreationReturnCodes;

use base64::engine::{general_purpose::STANDARD, Engine};
use rand::{rng, Rng};
use rand::distr::Alphanumeric;

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

    pub fn connect(&mut self, settings: &Settings) -> Result<()> {
        info!("Connecting to WebX Router...");

        // Create ZMQ context
        let context = zmq::Context::new();

        let connector_port = settings.transport.ports.connector;
        let connector_socket = self.create_req_socket(&context, connector_port, None)?;

        let response = self.send(&connector_socket, "comm")?;

        let comm_response = self.decode_comm_response(&response)?;

        info!("Got session port {}", comm_response.session_port);

        let session_socket = self.create_req_socket(&context, comm_response.session_port, Some(comm_response.public_key))?;

        let _ = self.session_socket.insert(SessionSocket { port: comm_response.session_port, socket: session_socket });

        self.disconnect_req_socket(&connector_socket, connector_port);

        Ok(())
    }

    pub fn disconnect(&mut self) {
        if let Some(session_socket) = self.session_socket.as_ref() {
            self.disconnect_req_socket(&session_socket.socket, session_socket.port);
        }
    }

    pub fn create(&self, width: u32, height: u32, keyboard_layout: &str) -> Result<CreationResponse> {
        let session_socket = self.session_socket.as_ref().ok_or_else(|| RouterError::SystemError(format!("Session Socket is unavailable")))?;

        // Create credentials file in the user's home directory (ro by the user)

        // create a random password

        // Write the password to the file

        let create_request = format!("create,{},{},{},{},{}", self.encode_base64("/tmp/test"), self.encode_base64("password"), width, height, keyboard_layout);

        let response = self.send(&session_socket.socket, &create_request)?;

        self.decode_create_response(&response)
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

