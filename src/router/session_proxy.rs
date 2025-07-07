use crate::common::{Result, RouterError, Settings, EventBus, INPROC_APP_TOPIC, INPROC_SESSION_TOPIC, APPLICATION_SHUTDOWN_COMMAND};
use crate::authentication::{Authenticator, AuthenticatedSession, Credentials};
use crate::engine::{EngineSessionManager, SessionConfig};
use crate::sesman::ScreenResolution;

use std::str;
use std::process;
use std::vec::Vec;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::{thread, time};
use base64::engine::{general_purpose::STANDARD, Engine};

/// The `SessionProxy` manages session-related requests such as requesting a new X11 session from the WebX Session Manager (using
/// credentials passed by the client), removing an existing session, connecting a client to an existing session, disconnecting a client from a session
/// and pinging a session to check if it is still active. 
/// It runs in a separate thread listening to requests from the WebX Relay.
pub struct SessionProxy {
    context: zmq::Context,
    authenticator: Authenticator,
    engine_session_manager: Arc<Mutex<EngineSessionManager>>,
    is_running: Arc<AtomicBool>,
}

#[repr(u32)]
pub enum SessionCreationReturnCodes {
    Success = 0,
    InvalidRequestParameters = 1,
    CreationError = 2,
    AuthenticationError = 3,
}

impl SessionCreationReturnCodes {
    pub fn to_u32(self) -> u32 {
        self as u32
    }

    pub fn try_from(value: u32) -> Result<Self> {
        match value {
            0 => Ok(SessionCreationReturnCodes::Success),
            1 => Ok(SessionCreationReturnCodes::InvalidRequestParameters),
            2 => Ok(SessionCreationReturnCodes::CreationError),
            3 => Ok(SessionCreationReturnCodes::AuthenticationError),
            _ => Err(RouterError::SystemError(format!("Failed to convert SessionCreationReturnCode {}", value))),
        }
    }

}

impl SessionProxy {
    /// Creates a new `SessionProxy` instance.
    ///
    /// # Arguments
    /// * `context` - The ZeroMQ context.
    pub fn new(context: zmq::Context, settings: &Settings) -> Self {
        let context_clone = context.clone();
        Self {
            context,
            authenticator: Authenticator::new(settings.sesman.authentication.service.to_owned()),
            engine_session_manager: Arc::new(Mutex::new(EngineSessionManager::new(settings, context_clone))),
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Runs the session proxy, handling incoming requests and events.
    ///
    /// # Arguments
    /// * `settings` - The application settings.
    ///
    /// # Returns
    /// A result indicating success or failure.
    pub fn run(&mut self, settings: &Settings, secret_key: &str) -> Result<()> {
        let transport = &settings.transport;

        let secure_rep_socket = self.create_secure_rep_socket(transport.ports.session, secret_key)?;

        let event_bus_sub_socket = EventBus::create_event_subscriber(&self.context, &[INPROC_APP_TOPIC, INPROC_SESSION_TOPIC])?;

        // Create the thread to update session creations
        self.create_session_startup_thread();

        let mut items = [
            event_bus_sub_socket.as_poll_item(zmq::POLLIN),
            secure_rep_socket.as_poll_item(zmq::POLLIN),
        ];

        self.is_running.store(true, Ordering::SeqCst);
        while self.is_running.load(Ordering::SeqCst) {
            // Poll both sockets
            if zmq::poll(&mut items, 5000).is_ok() {
                // Check for event bus messages
                if items[0].is_readable() {
                    self.read_event_bus(&event_bus_sub_socket);
                }

                // Check for session REQ messages (if running)
                if items[1].is_readable() && self.is_running.load(Ordering::SeqCst) {
                    self.handle_secure_request(&secure_rep_socket);
                }
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
    fn create_secure_rep_socket(&self, port: u32, secret_key: &str) -> Result<zmq::Socket> {
        let socket = self.context.socket(zmq::REP)?;
        socket.set_linger(0)?;

        // Secure the socket 
        let secret_key = zmq::z85_decode(secret_key)?;
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
                self.is_running.store(false, Ordering::SeqCst);

                // Close all sessions gracefully
                if let Ok(mut engine_session_manager) = self.engine_session_manager.lock() {
                    engine_session_manager.shutdown();
                } else {
                    error!("Failed to lock EngineSessionManager for shutdown");
                };

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
    fn handle_secure_request(&mut self, secure_rep_socket: &zmq::Socket) {
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
                let secret = message_parts[1];

                // Ping the session and get a string response
                let ping_response = self.ping_engine(&secret);
                if let Err(error) = secure_rep_socket.send(ping_response.as_str(), 0) {
                    error!("Failed to send session ping message: {}", error);
                }
            }
            send_empty = false;

        } else if message_parts[0] == "status" {
            // Verify that we have a sessionId
            if message_parts.len() < 2 {
                error!("Received invalid status command");

            } else {
                let secret = message_parts[1];

                // Get the status of the session (starting or ready)
                let status_response = self.get_session_status(&secret);
                if let Err(error) = secure_rep_socket.send(status_response.as_str(), 0) {
                    error!("Failed to send session status message: {}", error);
                }
                send_empty = false;
            }

        } else if message_parts[0] == "create" || message_parts[0] == "create_async" {
            let is_async = message_parts[0] == "create_async";
            match self.decode_create_command(&message_parts) {
                Ok((username, password, session_config)) => {

                    let credentials = match Credentials::new(username, password) {
                        Ok(credentials) => credentials,
                        Err(err) => {
                            if let Err(error) = secure_rep_socket.send(format!("{},{}", SessionCreationReturnCodes::AuthenticationError.to_u32(), err).as_str(), 0) {
                                error!("Failed to send session creation error response: {}", error);
                            }
                            return;
                        }
                    };

                    info!("Got session create command for user \"{}\"", credentials.username());

                    // Authenticate the user and create a session
                    let authenticed_session = match self.authenticator.authenticate(&credentials) {
                        Ok(authenticated_session) => authenticated_session,
                        Err(error) => {
                            error!("Failed to authenticate user {}: {}", credentials.username(), error);
                            if let Err(error) = secure_rep_socket.send(format!("{},{}", SessionCreationReturnCodes::AuthenticationError.to_u32(), error).as_str(), 0) {
                                error!("Failed to send session creation error response: {}", error);
                            }
                            return;
                        }
                    };

                    info!("Successfully authenticated user: \"{}\"", &credentials.username());

                    // Request session from WebX Session Manager
                    let message = if is_async {
                        self.get_or_create_session_async(authenticed_session, session_config)
                    } else {
                        self.get_or_create_session(authenticed_session, session_config)
                    };

                    // Send message response
                    if let Err(error) = secure_rep_socket.send(message.as_str(), 0) {
                        error!("Failed to send session creation response: {}", error);
                    }
                    send_empty = false;
                },
                Err(error) => {
                    error!("Failed to decode create command: {}", error);
                    
                    // Send error response
                    if let Err(error) = secure_rep_socket.send(format!("{},{}", SessionCreationReturnCodes::InvalidRequestParameters.to_u32(), error).as_str(), 0) {
                        error!("Failed to send session creation error response: {}", error);
                    }
                    send_empty = false;
                }
            }

        } else if message_parts[0] == "list" {
            if let Ok(engine_session_manager) = self.engine_session_manager.lock() {
                // Debug output of all X11 sessions
                let all_x11_sessions = engine_session_manager.get_all_x11_sessions().iter().map(|session| 
                    format!("id={},width={},height={},username={},uid={}", 
                        session.id(),
                        session.resolution().width(),
                        session.resolution().height(),
                        session.account().username(),
                        session.account().uid()),
                    ).collect::<Vec<String>>().join("\n");
                debug!("All X11 sessions:\n{}", all_x11_sessions);

                if let Err(error) = secure_rep_socket.send(all_x11_sessions.as_str(), 0) {
                    error!("Failed to send list of all sessions: {}", error);
                }
                send_empty = false;
            } else {
                error!("Failed to lock EngineSessionManager to list sessions");
            }

        } else if message_parts[0] == "connect" {

            // Verify that we have a sessionId
            if message_parts.len() < 2 {
                error!("Received invalid connect command");

            } else {
                let secret = message_parts[1];

                if let Ok(mut engine_session_manager) = self.engine_session_manager.lock() {
                    // Forward the connection request
                    match engine_session_manager.send_engine_request(&secret, &message_text) {
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
                } else {
                    error!("Failed to lock EngineSessionManager to connect session");
                }
            }

        } else if message_parts[0] == "disconnect" {

            // Verify that we have a sessionId
            if message_parts.len() < 3 {
                error!("Received invalid disconnect command");

            } else {
                let secret = message_parts[1];

                if let Ok(mut engine_session_manager) = self.engine_session_manager.lock() {
                    // Forward the disconnection request
                    match engine_session_manager.send_engine_request(&secret, &message_text) {
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
                } else {
                    error!("Failed to lock EngineSessionManager to disconnect session");
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

    /// Retrieves or creates a session synchronously and returns its secret.
    ///
    /// # Arguments
    /// * `authenticated_session` - The authenticated user session (account and environment).
    /// * `session_config` - The session config (screen resolution, keyboard layout, additional parameters).
    ///
    /// # Returns
    /// * `String` - The session creation result as a string (success or error code and message).
    fn get_or_create_session(&mut self, authenticated_session: AuthenticatedSession, session_config: SessionConfig) -> String {
        let username = authenticated_session.account().username().to_string();
        
        if let Ok(mut engine_session_manager) = self.engine_session_manager.lock() {
            let timeout = time::Duration::from_secs(15);
            match engine_session_manager.get_or_create_x11_and_engine_session(authenticated_session, session_config, timeout) {
                Ok(secret) => format!("{},{}", SessionCreationReturnCodes::Success.to_u32(), secret),
                Err(error) => {
                    error!("Failed to create session for user {}: {}", username, error);
                    match error {
                        RouterError::AuthenticationError(_) => {
                            format!("{},{}", SessionCreationReturnCodes::AuthenticationError.to_u32(), error)
                        },
                        _ => {
                            format!("{},{}", SessionCreationReturnCodes::CreationError.to_u32(), error)
                        }
                    }
                }
            }
        } else {
            error!("Failed to lock EngineSessionManager to create session for user {}", username);
            format!("{},{}", SessionCreationReturnCodes::CreationError.to_u32(), "Failed to lock EngineSessionManager")
        }
    }


    /// Retrieves or creates a session asynchronously and returns its secret and creation status (starting or running)
    ///
    /// # Arguments
    /// * `authenticated_session` - The authenticated user session (account and environment).
    /// * `session_config` - The session config (screen resolution, keyboard layout, additional parameters).
    ///
    /// # Returns
    /// * `String` - The session creation result as a string (success or error code and message).
    fn get_or_create_session_async(&mut self, authenticated_session: AuthenticatedSession, session_config: SessionConfig) -> String {
        let username = authenticated_session.account().username().to_string();
        if let Ok(mut engine_session_manager) = self.engine_session_manager.lock() {
            match engine_session_manager.get_or_create_x11_and_engine_session_async(authenticated_session, session_config) {
                Ok(engine_session_info) => {
                    format!("{},{},{}", SessionCreationReturnCodes::Success.to_u32(), engine_session_info.secret(), engine_session_info.status().to_u32())
                },
                Err(error) => {
                    error!("Failed to create session for user {}: {}", username, error);
                    match error {
                        RouterError::AuthenticationError(_) => {
                            format!("{},{}", SessionCreationReturnCodes::AuthenticationError.to_u32(), error)
                        },
                        _ => {
                            format!("{},{}", SessionCreationReturnCodes::CreationError.to_u32(), error)
                        }
                    }
                }
            }
        } else {
            error!("Failed to lock EngineSessionManager to create session for user {}", username);
            format!("{},{}", SessionCreationReturnCodes::CreationError.to_u32(), "Failed to lock EngineSessionManager")
        }
    }

    /// Pings a session to check if it is active.
    ///
    /// # Arguments
    /// * `secret` - The secret of the session to ping.
    ///
    /// # Returns
    /// * `String` - A string indicating the ping result ("pong" or "pang" with error).
    fn ping_engine(&mut self, secret: &str) -> String {
        if let Ok(mut engine_session_manager) = self.engine_session_manager.lock() {
            match engine_session_manager.ping_engine(secret) {
                Ok(_) => format!("pong,{}", secret),
                Err(error) => {
                    format!("pang,{},{}", secret, error)
                }
            }
        } else {
            error!("Failed to lock EngineSessionManager to ping session with secret {}", secret);
            format!("pang,{},Failed to lock EngineSessionManager", secret)
        }
    }


    /// Gets the status of a session.
    ///
    /// # Arguments
    /// * `secret` - The secret of the session to ping.
    ///
    /// # Returns
    /// * `String` - A string indicating the creation status of the session
    fn get_session_status(&self, secret: &str) -> String {
        if let Ok(engine_session_manager) = self.engine_session_manager.lock() {
            match engine_session_manager.get_session_status(secret) {
                Ok(engine_session_info) => {
                    format!("{},{}", secret, engine_session_info.status().to_u32())
                },
                Err(error) => {
                    format!("{}", error)
                }
            }
        } else {
            error!("Failed to lock EngineSessionManager to ping session with secret {}", secret);
            format!("pang,{},Failed to lock EngineSessionManager", secret)
        }
    }

    /// Decodes a session creation command.
    ///
    /// # Arguments
    /// * `message_parts` - The parts of the command message.
    ///
    /// # Returns
    /// * `Result<(String, String, u32, u32, String, HashMap<String, String>)>` - 
    ///   Ok with a tuple of username, password, width, height, keyboard, and engine parameters if successful, Err otherwise.
    fn decode_create_command(&self, message_parts: &Vec<&str>) -> Result<(String, String, SessionConfig)> {
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

            let session_config = SessionConfig::new(
                keyboard,
                ScreenResolution::new(width, height),
                engine_parameters,
            );
    
            Ok((username, password, session_config))

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
    /// * `Result<String>` - The decoded string if successful, Err otherwise.
    fn decode_base64(&self, input: &str) -> Result<String> {
        let decoded_bytes = STANDARD.decode(input)?;

        let output = str::from_utf8(&decoded_bytes)?;

        Ok(output.to_string())
    }


    /// Spawns a background thread that regularly updates session startup processes.
    /// This thread will keep running as long as `is_running` is true.
    fn create_session_startup_thread(&self) -> thread::JoinHandle<()> {
        let engine_session_manager = Arc::clone(&self.engine_session_manager);
        let is_running = Arc::clone(&self.is_running);

        thread::spawn({
            move || {
                while is_running.load(Ordering::SeqCst) {
                    if let Ok(mut engine_session_manager) = engine_session_manager.lock() {
                        // Check if there are any starting processes that need to be launched
                        engine_session_manager.update_starting_processes();
                    }

                    // Sleep for a while before checking again
                    thread::sleep(time::Duration::from_millis(500));
                }
            }
        })
    }
}
