use crate::common::*;
use crate::authentication::Credentials;
use crate::service::EngineSessionService;
use crate::sesman::ScreenResolution;

use std::str;
use std::process;
use std::vec::Vec;
use std::collections::HashMap;

use base64::engine::{general_purpose::STANDARD, Engine};

/// The `SessionProxy` manages session-related requests such as requesting a new X11 session from the WebX Session Manager (using
/// credentials passed by the client), removing an existing session, connecting a client to an existing session, disconnecting a client from a session
/// and pinging a session to check if it is still active. 
/// It runs in a separate thread listening to requests from the WebX Relay.
pub struct SessionProxy {
    context: zmq::Context,
    service: EngineSessionService,
    is_running: bool,
}

impl SessionProxy {
    /// Creates a new `SessionProxy` instance.
    ///
    /// # Arguments
    /// * `context` - The ZeroMQ context.
    pub fn new(context: zmq::Context, settings: &SesManSettings) -> Self {
        Self {
            context,
            service: EngineSessionService::new(settings),
            is_running: false,
        }
    }

    /// Runs the session proxy, handling incoming requests and events.
    ///
    /// # Arguments
    /// * `settings` - The application settings.
    ///
    /// # Returns
    /// A result indicating success or failure.
    pub fn run(&mut self, settings: &Settings) -> Result<()> {
        let transport = &settings.transport;

        let secure_rep_socket = self.create_secure_rep_socket(transport.ports.session, &transport.encryption.private)?;

        let event_bus_sub_socket = EventBus::create_event_subscriber(&self.context, &[INPROC_APP_TOPIC, INPROC_SESSION_TOPIC])?;

        let mut items = [
            event_bus_sub_socket.as_poll_item(zmq::POLLIN),
            secure_rep_socket.as_poll_item(zmq::POLLIN),
        ];

        self.is_running = true;
        while self.is_running {
            // Poll both sockets
            if zmq::poll(&mut items, 5000).is_ok() {
                // Check for event bus messages
                if items[0].is_readable() {
                    self.read_event_bus(&event_bus_sub_socket);
                }

                // Check for session REQ messages (if running)
                if items[1].is_readable() && self.is_running {
                    self.handle_secure_request(&secure_rep_socket, settings);
                }

                // Cleanup inactive sessions
                self.service.cleanup_inactive_engine_sessions(settings);
            }
        }

        debug!("Stopped Session Proxy");

        Ok(())
    }

    /// Creates a secure REP socket for handling session requests.
    ///
    /// # Arguments
    /// * `port` - The port to bind the socket to.
    /// * `secret_key_string` - The secret key for securing the socket.
    ///
    /// # Returns
    /// The created ZeroMQ socket.
    fn create_secure_rep_socket(&self, port: u32, secret_key_string: &str) -> Result<zmq::Socket> {
        let socket = self.context.socket(zmq::REP)?;
        socket.set_linger(0)?;

        // Secure the socket 
        let secret_key = zmq::z85_decode(secret_key_string)?;
        socket.set_curve_server(true)?;
        socket.set_curve_secretkey(&secret_key)?;

        let address = format!("tcp://*:{}", port);
        match socket.bind(address.as_str()) {
            Ok(_) => debug!("Session Proxy bound to {}", address),
            Err(error) => {
                error!("Failed to bind Session Proxy socket to {}: {}", address, error);
                process::exit(1);
            }
        }

        Ok(socket)
    }

    /// Reads and processes messages from the event bus.
    ///
    /// # Arguments
    /// * `event_bus_sub_socket` - The ZeroMQ subscription socket for the event bus.
    fn read_event_bus(&mut self, event_bus_sub_socket: &zmq::Socket) {
        let mut msg = zmq::Message::new();

        if let Err(error) = event_bus_sub_socket.recv(&mut msg, 0) {
            error!("Failed to receive event bus message: {}", error);

        } else {
            let event = msg.as_str().unwrap();
            if event == APPLICATION_SHUTDOWN_COMMAND {
                self.is_running = false;

                // Close all sessions gracefully
                self.service.shutdown();

            } else if event.starts_with(INPROC_SESSION_TOPIC) {
                let message_text = msg.as_str().unwrap();
                let message_parts = message_text.split(':').collect::<Vec<&str>>();
                let session_id = message_parts[1];
                self.service.update_engine_session_activity(&session_id);

            } else {
                warn!("Got unknown event bus command: {}", event);
            }
        }
    }

    /// Handles secure session requests. Requests are either forwarded to the WebX Session Manager to create/remove X11 sessions
    /// or forwarded to a specific WebX Engine.
    ///
    /// # Arguments
    /// * `secure_rep_socket` - The ZeroMQ REP socket for secure requests.
    /// * `settings` - The application settings.
    fn handle_secure_request(&mut self, secure_rep_socket: &zmq::Socket, settings: &Settings) {
        let mut msg = zmq::Message::new();

        // Get message on REQ socket
        if let Err(error) = secure_rep_socket.recv(&mut msg, 0) {
            error!("Failed to received message on session request socket: {}", error);
            return;
        }

        // Decode message
        let mut send_empty = true;
        let message_text = msg.as_str().unwrap();
        let message_parts = message_text.split(',').collect::<Vec<&str>>();

        if message_parts[0] == "ping" {

            // Check for router or engine ping
            if message_parts.len() == 1 {
                // Ping response for router
                if let Err(error) = secure_rep_socket.send("pong", 0) {
                    error!("Failed to send pong message: {}", error);
                }

            } else {
                let session_id = message_parts[1];
                trace!("Got ping for engine {}", session_id);

                // Ping the session and get a string response
                let ping_response = self.ping_engine(&session_id);
                if let Err(error) = secure_rep_socket.send(ping_response.as_str(), 0) {
                    error!("Failed to send session ping message: {}", error);
                }
            }
            send_empty = false;

        } else if message_parts[0] == "create" {
            match self.decode_create_command(&message_parts) {
                Ok((username, password, width, height, keyboard, engine_parameters)) => {
                    info!("Got session create command for user \"{}\"", username);

                    // Request session from WebX Session Manager
                    let message = self.get_or_create_session(settings, Credentials::new(username, password), ScreenResolution::new(width, height), &keyboard, &engine_parameters);

                    // Debug output of all X11 sessions
                    let all_x11_sessions = self.service.get_all_x11_sessions().map(|sessions| {
                        sessions.iter().map(|session| 
                            format!("id={},width={},height={},username={},uid={}", 
                                session.id(),
                                session.resolution().width(),
                                session.resolution().height(),
                                session.account().username(),
                                session.account().uid())).collect::<Vec<String>>().join("\n")
                    }).unwrap_or_default();
                    debug!("All X11 sessions:\n{}", all_x11_sessions);

                    // Send message response
                    if let Err(error) = secure_rep_socket.send(message.as_str(), 0) {
                        error!("Failed to send session creation response: {}", error);
                    }
                    send_empty = false;
                },
                Err(error) => {
                    error!("Failed to decode create command: {}", error);
                    
                    // Send error response
                    if let Err(error) = secure_rep_socket.send(format!("1,{}", error).as_str(), 0) {
                        error!("Failed to send session creation error response: {}", error);
                    }
                    send_empty = false;
                }
            }

        } else if message_parts[0] == "connect" {

            // Verify that we have a sessionId
            if message_parts.len() < 2 {
                error!("Received invalid connect command");

            } else {
                let session_id = message_parts[1];
                info!("Got connect for session {}", session_id);

                // Forward the connection request
                match self.service.send_engine_request(&session_id, &self.context, &message_text) {
                    Ok(response) => {
                        if let Err(error) = secure_rep_socket.send(response.as_str(), 0) {
                            error!("Failed to send client connection response: {}", error);
                        }
                        send_empty = false;
                    }
                    Err(error) => {
                        error!("Failed to send client connection request: {}", error);
                    }
                }
            }

        } else if message_parts[0] == "disconnect" {

            // Verify that we have a sessionId
            if message_parts.len() < 3 {
                error!("Received invalid disconnect command");

            } else {
                let session_id = message_parts[1];
                let client_id = message_parts[2];
                info!("Got disconnect from client {} for session {}", client_id, session_id);

                // Forward the disconnection request
                match self.service.send_engine_request(&session_id, &self.context, &message_text) {
                    Ok(response) => {
                        if let Err(error) = secure_rep_socket.send(response.as_str(), 0) {
                            error!("Failed to send client disconnection response: {}", error);
                        }
                        send_empty = false;
                    }
                    Err(error) => {
                        error!("Failed to send client disconnection request: {}", error);
                    }
                }
            }

        } else {
            error!("Got unknown session command: {}", message_parts[0]);
        }

        // If send needed then send empty message
        if send_empty {
            let empty_message = zmq::Message::new();
            if let Err(error) = secure_rep_socket.send(empty_message, 0) {
                error!("Failed to send empty message: {}", error);
            }
        }
    }

    /// Retrieves or creates a session and returns its ID.
    ///
    /// # Arguments
    /// * `settings` - The application settings.
    /// * `username` - The username of the user.
    /// * `password` - The password of the user.
    /// * `width` - The width of the session display.
    /// * `height` - The height of the session display.
    /// * `keyboard` - The keyboard layout.
    ///
    /// # Returns
    /// A string containing the session ID or an error message.
    fn get_or_create_session(&mut self, settings: &Settings, credentials: Credentials, resolution: ScreenResolution, keyboard: &str, engine_parameters: &HashMap<String, String>) -> String {
        match self.service.get_or_create_engine_session(settings, &credentials, resolution, keyboard, engine_parameters, &self.context) {
            Ok(session) => format!("0,{}", session.id()),
            Err(error) => {
                error!("Failed to create session for user {}: {}", credentials.username(), error);
                format!("1,{}", error)
            }
        }
    }

    /// Pings a session to check if it is active.
    ///
    /// # Arguments
    /// * `session_id` - The ID of the session to ping.
    ///
    /// # Returns
    /// A string indicating the ping result.
    fn ping_engine(&mut self, session_id: &str) -> String {
        match self.service.ping_engine(session_id, &self.context) {
            Ok(_) => format!("pong,{}", session_id),
            Err(error) => {
                error!("Failed to ping session with id {}: {}", session_id, error);
                format!("pang,{},{}", session_id, error)
            }
        }
    }

    /// Decodes a session creation command.
    ///
    /// # Arguments
    /// * `message_parts` - The parts of the command message.
    ///
    /// # Returns
    /// A tuple containing the decoded parameters or an error.
    fn decode_create_command(&self, message_parts: &Vec<&str>) -> Result<(String, String, u32, u32, String, HashMap<String, String>)> {
        if message_parts.len() >= 6 {
            let username_base64 = message_parts[1];
            let password_base64 = message_parts[2];
            let username = self.decode_base64(username_base64)?;
            let password = self.decode_base64(password_base64)?;

            let width = message_parts[3].to_string().parse::<u32>()?;
            let height = message_parts[4].to_string().parse::<u32>()?;
            let keyboard = message_parts[5].to_string();

            let mut engine_parameters = HashMap::new();
            if message_parts.len() > 6 {
                for param in message_parts.iter().skip(6) {
                    match param.split_once('=') {
                        Some((key, value)) => {
                            engine_parameters.insert(key.to_string(), value.to_string());
                        }
                        None => {
                            return Err(RouterError::EngineSessionError(format!("Failed to parse the engine parameter: {}", param)));
                        }
                    }
                }
                debug!("Parsed engine parameters: {:?}", engine_parameters);
            }
    
            Ok((username, password, width, height, keyboard, engine_parameters))

        } else {
            Err(RouterError::EngineSessionError(format!("Incorrect number of parameters. Got {}, expected 6", message_parts.len())))
        }
    }

    /// Decodes a Base64-encoded string.
    ///
    /// # Arguments
    /// * `input` - The Base64-encoded string.
    ///
    /// # Returns
    /// The decoded string.
    fn decode_base64(&self, input: &str) -> Result<String> {
        let decoded_bytes = STANDARD.decode(input)?;

        let output = str::from_utf8(&decoded_bytes)?;

        Ok(output.to_string())
    }
}
