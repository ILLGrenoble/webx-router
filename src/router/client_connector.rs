use crate::common::*;

pub struct ClientConnector {
    context: zmq:: Context,
}

impl ClientConnector {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context,
        }
    }

    pub fn run(&self, settings: &Settings) -> Result<()> {
        let transport = &settings.transport;

        // Create REP socket
        let rep_socket = self.create_rep_socket(transport.ports.connector)?;

        // Create event bus SUB
        let event_bus_sub_socket = EventBus::create_event_subscriber(&self.context, &[INPROC_APP_TOPIC])?;

        let mut is_running = true;
        while is_running {
            let mut msg = zmq::Message::new();

            let mut items = [
                event_bus_sub_socket.as_poll_item(zmq::POLLIN),
                rep_socket.as_poll_item(zmq::POLLIN),
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
                        } else {
                            warn!("Got unknown event bus message: {}", event);
                        }
                    }
                }

                // Check for REQ-REP message (if running)
                if items[1].is_readable() && is_running {
                    if let Err(error) = rep_socket.recv(&mut msg, 0) {
                        error!("Failed to received message on relay req-rep: {}", error);

                    } else {
                        let message_text = msg.as_str().unwrap();

                        if message_text == "comm" {
                            // Comm message
                            if let Err(error) = rep_socket.send(format!("{},{},{},{}", 
                                transport.ports.publisher, 
                                transport.ports.collector,
                                transport.ports.session,
                                transport.encryption.public).as_str(), 0) {
                                    error!("Failed to send comm message: {}", error);
                            }

                        } else if message_text == "ping" {
                            // Ping response
                            if let Err(error) = rep_socket.send("pong", 0) {
                                error!("Failed to send pong message: {}", error);
                            }

                        } else {
                            // If send needed then send empty message
                            let empty_message = zmq::Message::new();
                            if let Err(error) = rep_socket.send(empty_message, 0) {
                                error!("Failed to send empty message: {}", error);
                            }
                        }
                    }
                }
            }
        }

        debug!("Stopped Client Connector");

        Ok(())
    }

    fn create_rep_socket(&self, port: u32) -> Result<zmq::Socket> {
        let socket = self.context.socket(zmq::REP)?;
        socket.set_linger(0)?;

        let address = format!("tcp://*:{}", port);
        match socket.bind(address.as_str()) {
            Ok(_) => debug!("Client Connector bound to {}", address),
            Err(error) => return Err(RouterError::TransportError(format!("Failed to bind REP socket to {}: {}", address, error)))
        }

        Ok(socket)
    }

}
