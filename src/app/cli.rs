use crate::common::{Result, RouterError, random_string};
use crate::router::SessionCreationReturnCodes;
use crate::fs::chmod;

use base64::engine::{general_purpose::STANDARD, Engine};
use std::{
    fs::File,
    io::Write,
    thread,
    time,
    sync::{Mutex, Arc},
};
use std::time::{Duration, Instant};

/// Holds information about the communication response from the router.
struct CommResponse {
    pub _publisher_port: u32,
    pub _subscriber_port: u32,
    pub session_port: u32,
    pub public_key: String,
}

/// Represents the response to a session creation request.
pub struct CreationResponse {
    pub code: SessionCreationReturnCodes,
    pub message: String,
}

/// Holds information about a session socket, including its port and the ZMQ socket itself.
struct SessionSocket {
    pub port: u32,
    pub socket: zmq::Socket,
}

/// Main CLI struct for interacting with the WebX Router.
pub struct Cli {
    /// Optionally holds the current session socket.
    session_socket: Option<SessionSocket>,
}

impl Cli {
    /// Creates a new CLI instance with no session socket.
    ///
    /// # Returns
    /// A new `Cli` instance.
    pub fn new() -> Self {
        Self {
            session_socket: None,
        }
    }

    /// Connects to the WebX Router using the specified connector port.
    /// Sets up the session socket for further communication.
    ///
    /// # Arguments
    /// * `connector_port` - The port to connect to the WebX Router.
    ///
    /// # Returns
    /// * `Result<()>` - Ok if connection is successful, Err otherwise.
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

        // Create session socket using the session port and public key
        let session_socket = self.create_req_socket(&context, comm_response.session_port, Some(comm_response.public_key))?;
        let _ = self.session_socket.insert(SessionSocket { port: comm_response.session_port, socket: session_socket });

        // Disconnect the connector socket
        self.disconnect_req_socket(&connector_socket, connector_port);

        info!("Connected to WebX Router");

        Ok(())
    }

    /// Disconnects from the WebX Router by disconnecting the session socket if it exists.
    ///
    /// # Arguments
    /// None.
    ///
    /// # Returns
    /// Nothing.
    pub fn disconnect(&mut self) {
        if let Some(session_socket) = self.session_socket.as_ref() {
            self.disconnect_req_socket(&session_socket.socket, session_socket.port);
            debug!("Disconnected from WebX Router");
        }
    }

    /// Creates a new WebX Engine session with the specified parameters.
    /// Generates a credentials file, sends a creation request, and cleans up the credentials file.
    ///
    /// # Arguments
    /// * `width` - The width of the session screen.
    /// * `height` - The height of the session screen.
    /// * `keyboard_layout` - The keyboard layout to use.
    ///
    /// # Returns
    /// * `Result<CreationResponse>` - The response from the session creation request.
    pub fn create(&self, width: u32, height: u32, keyboard_layout: &str) -> Result<CreationResponse> {
        let session_socket = self.session_socket.as_ref().ok_or_else(|| RouterError::SystemError(format!("Session Socket is unavailable")))?;

        info!("Creating WebX Engine Session command with resolution {} x {} and keyboard layout {}", width, height, keyboard_layout);

        // Create credentials file in /tmp (readable only by the user)
        let credentials_path = format!("/tmp/{}", random_string(8));
        let mut file = File::create(&credentials_path)?;
        chmod(&credentials_path, 0o600)?;

        // Create and write a random password to the credentials file
        let password = random_string(32);
        file.write_all(password.as_bytes())?;

        debug!("Credentials written to {}", credentials_path);

        // Send the creation request to the WebX Router
        debug!("Sending creation request to WebX Router...");
        let create_request = format!("create,{},{},{},{},{}", self.encode_base64(&credentials_path), self.encode_base64(&password), width, height, keyboard_layout);
        let response = self.send(&session_socket.socket, &create_request)?;

        debug!("... received response {}", response);

        // Remove the credentials file after use
        std::fs::remove_file(&credentials_path)?;

        // Decode and return the creation response
        self.decode_create_response(&response)
    }

    /// Sends a list request to the WebX Router and returns the response as a string.
    ///
    /// # Returns
    /// * `Result<String>` - The response from the list request.
    pub fn list(&self) -> Result<String> {
        let session_socket = self.session_socket.as_ref().ok_or_else(|| RouterError::SystemError(format!("Session Socket is unavailable")))?;

        debug!("Sending list request to WebX Router...");
        let response = self.send(&session_socket.socket, "list")?;

        debug!("... received response {}", response);

        Ok(response)
    }

    /// Waits for a Ctrl-C interrupt, sending periodic pings to the WebX Router.
    /// Exits when Ctrl-C is received or the engine session is no longer running.
    ///
    /// # Arguments
    /// * `session_id` - The session ID to ping.
    ///
    /// # Returns
    /// * `Result<()>` - Ok if the loop exits cleanly, Err if ping fails.
    pub fn wait_for_interrupt(&self, session_id: &str) -> Result<()> {
        let session_socket = self.session_socket.as_ref().ok_or_else(|| RouterError::SystemError(format!("Session Socket is unavailable")))?;

        // Shared flag to indicate if the process should keep running
        let running_mutex = Arc::new(Mutex::new(true));
        let mut is_running = true;

        // Set up Ctrl-C handler to set the running flag to false
        let running_handler = Arc::clone(&running_mutex);
        ctrlc::set_handler(move || {
            debug!("CTRL-C received");
            if let Ok(mut running) = running_handler.lock() {
               *running = false;
            }
        }).expect("Error setting Ctrl-C handler");

        let mut last_ping = Instant::now();
        let ping_request = format!("ping,{}", session_id);

        // Main loop: sleep, send pings every 5 seconds, and check running flag
        while is_running {
            thread::sleep(time::Duration::from_millis(100));

            // Every 5 seconds, send a ping to the WebX Router
            if last_ping.elapsed() >= Duration::from_secs(5) {
                debug!("Sending ping request to WebX Router...");
                let ping_response = self.send(&session_socket.socket, &ping_request)?;
                debug!("... received response {}", ping_response);
                last_ping = Instant::now();

                // If ping fails, exit with error
                if !self.decode_ping_response(&ping_response) {
                    return Err(RouterError::EngineSessionError(format!("Failed to ping engine")));
                }
            }

            // Update is_running from the mutex
            if let Ok(running) = running_mutex.lock() {
                is_running = *running;
            }
        }

        Ok(())
    }

    /// Decodes the communication response string into a CommResponse struct.
    ///
    /// # Arguments
    /// * `response` - The response string to decode.
    ///
    /// # Returns
    /// * `Result<CommResponse>` - The decoded communication response.
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

    /// Decodes the creation response string into a CreationResponse struct.
    ///
    /// # Arguments
    /// * `response` - The response string to decode.
    ///
    /// # Returns
    /// * `Result<CreationResponse>` - The decoded creation response.
    fn decode_create_response(&self, response: &str) -> Result<CreationResponse> {
        let response_parts = response.split(',').collect::<Vec<&str>>();
        let response_code_num: u32 = response_parts[0].parse()?;
        let message = response_parts[1].to_string();
        let code = SessionCreationReturnCodes::try_from(response_code_num)?;

        Ok( CreationResponse { code, message })
    }

    /// Decodes the ping response string and returns true if it is "pong".
    ///
    /// # Arguments
    /// * `response` - The response string to decode.
    ///
    /// # Returns
    /// * `bool` - True if the response is "pong", false otherwise.
    fn decode_ping_response(&self, response: &str) -> bool {
        let response_parts = response.split(',').collect::<Vec<&str>>();

        response_parts[0] == "pong"
    }

    /// Sends a request string over the given ZMQ socket and returns the response as a String.
    ///
    /// # Arguments
    /// * `socket` - The ZMQ socket to send the request on.
    /// * `request` - The request string to send.
    ///
    /// # Returns
    /// * `Result<String>` - The response from the socket.
    fn send(&self, socket: &zmq::Socket, request: &str) -> Result<String> {
        socket.send(request, 0)?;
        let mut message = zmq::Message::new();
        socket.recv(&mut message, 0)?;

        let text: String = message.as_str().unwrap().to_string();

        Ok(text)
    }

    /// Creates a ZMQ REQ socket, optionally securing it with a public key.
    ///
    /// # Arguments
    /// * `context` - The ZMQ context to use.
    /// * `port` - The port to connect to.
    /// * `public_key_option` - Optional public key for CurveZMQ security.
    ///
    /// # Returns
    /// * `Result<zmq::Socket>` - The created socket or an error if setup fails.
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

    /// Disconnects the given ZMQ socket from the specified port.
    ///
    /// # Arguments
    /// * `socket` - The ZMQ socket to disconnect.
    /// * `port` - The port to disconnect from.
    fn disconnect_req_socket(&self, socket: &zmq::Socket, port: u32) {
        let address = format!("tcp://localhost:{}", port);
        if let Err(error) = socket.disconnect(&address) {
            warn!("Failed to disconnect from Engine Connector socket at {}: {}", address, error)
        }
    }

    /// Encodes the input string as base64 using the standard engine.
    ///
    /// # Arguments
    /// * `input` - The string to encode.
    ///
    /// # Returns
    /// * `String` - The base64-encoded string.
    fn encode_base64(&self, input: &str) -> String {
        STANDARD.encode(input)
    }

}