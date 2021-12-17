use crate::common::*;
use crate::service::SessionService;

use std::process;
use std::vec::Vec;

pub struct SessionProxy {
    context: zmq::Context,
    service: SessionService,
}

impl SessionProxy {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context,
            service: SessionService::new()
        }
    }

    pub fn run(&mut self, settings: &Settings) -> Result<()> {
        let transport = &settings.transport;

        let secure_rep_socket = self.create_secure_rep_socket(transport.ports.session, &transport.encryption.private)?;

        let event_bus_sub_socket = EventBus::create_event_subscriber(&self.context, &[INPROC_APP_TOPIC])?;

        let mut is_running = true;
        while is_running {
            let mut msg = zmq::Message::new();

            let mut items = [
                event_bus_sub_socket.as_poll_item(zmq::POLLIN),
                secure_rep_socket.as_poll_item(zmq::POLLIN),
            ];

            // Poll both sockets
            if let Ok(_) = zmq::poll(&mut items, -1) {
                // Check for event bus messages
                if items[0].is_readable() {
                    if let Err(error) = event_bus_sub_socket.recv(&mut msg, 0) {
                        error!("Failed to receive event bus message: {}", error);

                    } else {
                        let event = msg.as_str().unwrap();
                        if event == APPLICATION_SHUTDOWN_COMMAND {
                            is_running = false;

                            // Close all sessions gracefully
                            self.service.stop_sessions();

                        } else {
                            warn!("Got unknown event bus command: {}", event);
                        }
                    }
                }

                // Check for session REQ messages (if running)
                if items[1].is_readable() && is_running {
                    // Get message on REQ socket
                    let mut send_empty = true;
                    if let Err(error) = secure_rep_socket.recv(&mut msg, 0) {
                        error!("Failed to received message on session request socket: {}", error);

                    } else {
                        // Decode message
                        let message_text = msg.as_str().unwrap();

                        if message_text == "ping" {
                            // Ping response
                            if let Err(error) = secure_rep_socket.send("pong", 0) {
                                error!("Failed to send pong message: {}", error);
                            }
                            send_empty = false;

                        } else {
                            let session_parameters = message_text.split(",").collect::<Vec<&str>>();
                            if session_parameters[0] == "create" {
                                if session_parameters.len() == 3 {
                                    let username = session_parameters[1];
                                    let password = session_parameters[2];
                                    info!("Got session create command with username \"{}\" and password \"{}\"", username, password);

                                    // Request session from WebX Session Manager
                                    let message = self.create_session(settings, username, password);
                                    if let Err(error) = secure_rep_socket.send(message.as_str(), 0) {
                                        error!("Failed to send session creation response: {}", error);
                                    }
                                    send_empty = false;
    
                                } else {
                                    error!("Got incorrect number of session create parameters. Got {}, expected 3", session_parameters.len());
                                }
    
                            } else {
                                error!("Got unknown session command");
                            }
                        }

                        if send_empty {
                            // If send needed then send empty message
                            let empty_message = zmq::Message::new();
                            if let Err(error) = secure_rep_socket.send(empty_message, 0) {
                                error!("Failed to send empty message: {}", error);
                            }
                        }
                        
                    }
                }
            }
        }

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

    fn create_session(&mut self, settings: &Settings, username: &str, password: &str) -> String {
        match self.service.create_session(settings, username, password) {
            Ok(session) => {
                format!("0,{}", session.id.to_simple())
            },
            Err(error) => {
                error!("Failed to create session for user {}: {}", username, error);
                format!("1,{}", error)
            }
        }
    }
}
