use crate::message_bus::*;
use crate::common::*;

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

        let message_bus_sub_socket = MessageBus::create_message_subscriber(&self.context, &[INPROC_APP_TOPIC, INPROC_SESSION_TOPIC]).unwrap();

        let mut is_running = true;
        while is_running {
            let mut msg = zmq::Message::new();

            let mut items = [
                message_bus_sub_socket.as_poll_item(zmq::POLLIN),
                relay_pull_socket.as_poll_item(zmq::POLLIN),
            ];

            // Poll both sockets
            if let Ok(_) = zmq::poll(&mut items, -1) {
                // Check for message_bus messages
                if items[0].is_readable() {
                    if let Err(error) = message_bus_sub_socket.recv(&mut msg, 0) {
                        error!("Failed to receive message_bus message: {}", error);

                    } else {
                        let message_bus_message = msg.as_str().unwrap();
                        if message_bus_message == APPLICATION_SHUTDOWN_COMMAND {
                            is_running = false;

                        } else if message_bus_message.starts_with(INPROC_SESSION_TOPIC) {
                            info!("Got message bus session command: {}", message_bus_message);

                        } else {
                            warn!("Got unknown message bus command: {}", message_bus_message);
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
