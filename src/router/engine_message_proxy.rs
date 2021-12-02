use crate::utils::*;
use crate::router::common::*;

pub struct EngineMessageProxy {
    context: zmq::Context
}

impl EngineMessageProxy {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context: context
        }
    }

    pub fn run(&self) {
        let relay_publisher_socket = self.create_relay_publisher_socket(RELAY_PUBLISHER_PORT).unwrap();

        let engine_subscriber_socket = self.create_engine_subscriber_socket(ENGINE_PUB_SUB_ADDR).unwrap();

        let event_bus_sub_socket = EventBus::create_event_subscriber(&self.context, &[INPROC_APP_TOPIC]).unwrap();

        let mut is_running = true;
        while is_running {
            let mut msg = zmq::Message::new();

            let mut items = [
                event_bus_sub_socket.as_poll_item(zmq::POLLIN),
                engine_subscriber_socket.as_poll_item(zmq::POLLIN),
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

                // Check for engine SUB messages (if running)
                if items[1].is_readable() && is_running {
                    // Get message on subscriber socket
                    if let Err(error) = engine_subscriber_socket.recv(&mut msg, 0) {
                        error!("Failed to received message from engine message publisher: {}", error);

                    } else {
                        debug!("Got message from engine of length {}", msg.len());
                        // Resend message on publisher socket
                        if let Err(error) = relay_publisher_socket.send(msg, 0) {
                            error!("Failed to send message to relay message subscriber: {}", error);
                        }   
                    }
                }
            }
        }

        info!("Stopped Engine Message Proxy");
    }

    fn create_relay_publisher_socket(&self, port: i32) -> Option<zmq::Socket> {
        let socket = self.context.socket(zmq::PUB).unwrap();
        socket.set_linger(0).unwrap();
        let address = format!("tcp://*:{}", port);
        if let Err(error) = socket.bind(address.as_str()) {
            error!("Failed to bind PUB socket to {}: {}", address, error);
            return None;
        }

        Some(socket)
    }

    fn create_engine_subscriber_socket(&self, address: &str) -> Option<zmq::Socket> {
        let socket = self.context.socket(zmq::SUB).unwrap();
        // Listen on all topics
        socket.set_subscribe(b"").unwrap();
        socket.set_linger(0).unwrap();
        if let Err(error) = socket.bind(address) {
            error!("Failed to bind engine SUB socket to {}: {}", address, error);
            return None;
        }

        Some(socket)
    }
}
