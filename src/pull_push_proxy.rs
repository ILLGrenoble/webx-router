use crate::process_communicator::{ProcessCommunicator, SHUTDOWN_COMMAND, ENGINE_PULL_PUSH_ADDR, RELAY_COLLECTOR_PORT};

pub struct PullPushProxy {
    context: zmq::Context
}

impl PullPushProxy {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context: context
        }
    }

    pub fn run(&self) {
        let relay_pull_socket = self.create_relay_pull_socket(RELAY_COLLECTOR_PORT).unwrap();

        let engine_push_socket = self.create_engine_push_socket(ENGINE_PULL_PUSH_ADDR).unwrap();

        let inproc_sub_socket = ProcessCommunicator::create_inproc_subscriber(&self.context).unwrap();

        let mut is_running = true;
        while is_running {
            let mut msg = zmq::Message::new();

            let mut items = [
                inproc_sub_socket.as_poll_item(zmq::POLLIN),
                relay_pull_socket.as_poll_item(zmq::POLLIN),
            ];

            // Poll both sockets
            if let Ok(_) = zmq::poll(&mut items, -1) {
                // Check for shutdown message
                if items[0].is_readable() {
                    if let Err(error) = inproc_sub_socket.recv(&mut msg, 0) {
                        error!("Failed to receive shutdown message: {}", error);

                    } else {
                        if msg.as_str().unwrap() == SHUTDOWN_COMMAND {
                            is_running = false;
                        }
                    }
                }

                // Check for relay PUSH messages (if running)
                if items[1].is_readable() && is_running {
                    // Get message on relay pull socket
                    if let Err(error) = relay_pull_socket.recv(&mut msg, 0) {
                        error!("Failed to received message on engine push: {}", error);

                    } else {
                        debug!("Got message from relay of length {}", msg.len());
                        // Resend message on engine push socket
                        if let Err(error) = engine_push_socket.send(msg, 0) {
                            error!("Failed to send message on relay pull: {}", error);
                        }   
                    }
                }
            }
        }

        info!("Stopped Pull-Push Proxy");
    }

    fn create_relay_pull_socket(&self, port: i32) -> Option<zmq::Socket> {
        let socket = self.context.socket(zmq::PULL).unwrap();
        socket.set_linger(0).unwrap();
        let address = format!("tcp://*:{}", port);
        if let Err(error) = socket.bind(address.as_str()) {
            error!("Failed to bind PULL socket to {}: {}", address, error);
            return None;
        }

        Some(socket)
    }

    fn create_engine_push_socket(&self, address: &str) -> Option<zmq::Socket> {
        let socket = self.context.socket(zmq::PUSH).unwrap();

        // TODO: all push clients become dependent on each other: PUSH waits for delivery so one client can block the others.
        // Have to add a timeout to the send so that clients that are no longer listening do not block the router.
        socket.set_sndtimeo(100).unwrap();
        
        socket.set_linger(0).unwrap();
        if let Err(error) = socket.bind(address) {
            error!("Failed to bind engine PUSH socket to {}: {}", address, error);
            return None;
        }

        Some(socket)
    }
}
