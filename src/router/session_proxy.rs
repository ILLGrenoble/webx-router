use crate::common::*;
use crate::service::SessionService;

use std::str;
use std::process;
use std::vec::Vec;

pub struct SessionProxy {
    context: zmq::Context,
    service: SessionService,
    is_running: bool,
}

impl SessionProxy {

    pub fn new(context: zmq::Context) -> Self {
        let cloned_context = context.clone();
        Self {
            context,
            service: SessionService::new(cloned_context),
            is_running: false,
        }
    }

    pub fn run(&mut self, settings: &Settings) -> Result<()> {
        let transport = &settings.transport;

        let secure_rep_socket = self.create_secure_rep_socket(transport.ports.session, &transport.encryption.private)?;

        let event_bus_sub_socket = EventBus::create_event_subscriber(&self.context, &[INPROC_APP_TOPIC])?;

        self.service.connect_to_sesman(settings)?;

        let mut items = [
            event_bus_sub_socket.as_poll_item(zmq::POLLIN),
            secure_rep_socket.as_poll_item(zmq::POLLIN),
        ];

        self.is_running = true;
        while self.is_running {
            // Poll both sockets
            if zmq::poll(&mut items, -1).is_ok() {
                // Check for event bus messages
                if items[0].is_readable() {
                    self.read_event_bus(&event_bus_sub_socket);
                }

                // Check for session REQ messages (if running)
                if items[1].is_readable() && self.is_running {
                    self.handle_secure_request(&secure_rep_socket, settings);
                }
            }
        }

        self.service.disconnect_from_sesman();

        debug!("Stopped Session Proxy");

        Ok(())
    }

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

    fn read_event_bus(&mut self, event_bus_sub_socket: &zmq::Socket) {
        let mut msg = zmq::Message::new();

        if let Err(error) = event_bus_sub_socket.recv(&mut msg, 0) {
            error!("Failed to receive event bus message: {}", error);

        } else {
            let event = msg.as_str().unwrap();
            if event == APPLICATION_SHUTDOWN_COMMAND {
                self.is_running = false;

                // Close all sessions gracefully
                self.service.stop_sessions();

            } else {
                warn!("Got unknown event bus command: {}", event);
            }
        }
    }

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
            // Ping response
            if let Err(error) = secure_rep_socket.send("pong", 0) {
                error!("Failed to send pong message: {}", error);
            }
            send_empty = false;

        } else if message_parts[0] == "create" {
            match self.decode_create_command(&message_parts) {
                Ok((username, password)) => {
                    info!("Got session create command with username \"{}\" and password \"{}\"", username, password);

                    // Request session from WebX Session Manager
                    let message = self.create_session(settings, &username, &password);

                    // Send message response
                    if let Err(error) = secure_rep_socket.send(message.as_str(), 0) {
                        error!("Failed to send session creation response: {}", error);
                    }
                    send_empty = false;
                },
                Err(error) => {
                    error!("Failed to decode create command: {}", error);
                }
            }

        } else {
            error!("Got unknown session command");
        }

        // If send needed then send empty message
        if send_empty {
            let empty_message = zmq::Message::new();
            if let Err(error) = secure_rep_socket.send(empty_message, 0) {
                error!("Failed to send empty message: {}", error);
            }
        }
    }

    fn create_session(&mut self, settings: &Settings, username: &str, password: &str) -> String {
        match self.service.create_session(settings, username, password, &self.context) {
            Ok(session) => format!("0,{}", session.id.to_simple()),
            Err(error) => {
                error!("Failed to create session for user {}: {}", username, error);
                format!("1,{}", error)
            }
        }
    }

    fn decode_create_command(&self, message_parts: &Vec<&str>) -> Result<(String, String)> {
        if message_parts.len() == 3 {
            let username_base64 = message_parts[1];
            let password_base64 = message_parts[2];
            let username = self.decode_base64(username_base64)?;
            let password = self.decode_base64(password_base64)?;

            Ok((username, password))

        } else {
            Err(RouterError::SessionError(format!("Incorrect number of parameters. Got {}, expected 3", message_parts.len())))
        }
    }

    fn decode_base64(&self, input: &str) -> Result<String> {
        let decoded_bytes = base64::decode(input)?;

        let output = str::from_utf8(&decoded_bytes)?;

        Ok(output.to_string())
    }
}
