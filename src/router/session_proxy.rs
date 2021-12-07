use crate::common::*;
use std::process;

pub struct SessionProxy {
    context: zmq::Context,
    port: u32,
    address: String
}

impl SessionProxy {

    pub fn new(context: zmq::Context, port: u32, address: String) -> Self {
        Self {
            context,
            port,
            address
        }
    }

    pub fn run(&self) -> Result<()> {
        let secure_rep_socket = self.create_secure_rep_socket(self.port)?;

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

                        } else {
                            warn!("Got unknown event bus command: {}", event);
                        }
                    }
                }

                // Check for session REQ messages (if running)
                if items[1].is_readable() && is_running {
                    // Get message on REQ socket
                    if let Err(error) = secure_rep_socket.recv(&mut msg, 0) {
                        error!("Failed to received message on session request socket: {}", error);

                    } else {
                        debug!("Got session request message of length {}", msg.len());

                        // TODO
                    }
                }
            }
        }

        info!("Stopped Session Proxy");

        Ok(())
    }

    fn create_secure_rep_socket(&self, port: u32) -> Result<zmq::Socket> {
        let socket = self.context.socket(zmq::REP)?;
        socket.set_linger(0)?;

        // Secure the socket
        let server_pair = zmq::CurveKeyPair::new()?;
        socket.set_curve_server(true)?;
        socket.set_curve_secretkey(&server_pair.secret_key)?;
        let public_key_string = zmq::z85_encode(&server_pair.public_key).unwrap();
        info!("public key : {}", public_key_string);

        let address = format!("tcp://*:{}", settings.ports.session);
        match socket.bind(address.as_str()) {
            Ok(_) => info!("Session Proxy bound to {}", address),
            Err(error) => {
                error!("Failed to bind Session Proxy socket to {}: {}", address, error);
                process::exit(1);
            }
        }

        Ok(socket)
    }
}
