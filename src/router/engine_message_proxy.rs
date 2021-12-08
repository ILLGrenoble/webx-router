use crate::common::*;
use std::process;

pub struct EngineMessageProxy {
    context: zmq::Context,
}

impl EngineMessageProxy {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context,
        }
    }

    pub fn run(&self, settings: &TransportSettings) -> Result<()> {
        let relay_publisher_socket = self.create_relay_publisher_socket(settings.ports.publisher)?;

        let engine_subscriber_socket = self.create_engine_subscriber_socket(&settings.ipc.message_proxy)?;

        let event_bus_sub_socket = EventBus::create_event_subscriber(&self.context, &[INPROC_APP_TOPIC])?;

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

        Ok(())
    }

    fn create_relay_publisher_socket(&self, port: u32) -> Result<zmq::Socket> {
        let socket = self.context.socket(zmq::PUB)?;
        socket.set_linger(0)?;
        let address = format!("tcp://*:{}", port);
        match socket.bind(address.as_str()) {
            Ok(_) => info!("Message Proxy bound to {}", address),
            Err(error) => {
                error!("Failed to bind PUB socket to {}: {}", address, error);
                process::exit(1);
            }
        }

        Ok(socket)
    }

    fn create_engine_subscriber_socket(&self, address: &String) -> Result<zmq::Socket> {
        let socket = self.context.socket(zmq::SUB)?;
        // Listen on all topics
        socket.set_subscribe(b"")?;
        socket.set_linger(0)?;
        if let Err(error) = socket.bind(address) {
            error!("Failed to bind engine SUB socket to {}: {}", address, error);
            process::exit(1);
        }
        Ok(socket)
    }
}
