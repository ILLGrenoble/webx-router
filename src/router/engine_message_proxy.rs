use crate::common::*;
use std::process;

pub struct EngineMessageProxy {
    context: zmq::Context,
    is_running: bool,
}

impl EngineMessageProxy {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context,
            is_running: false,
        }
    }

    pub fn run(&mut self, settings: &Settings) -> Result<()> {
        let transport = &settings.transport;
        
        let relay_publisher_socket = self.create_relay_publisher_socket(transport.ports.publisher)?;

        let engine_subscriber_socket = self.create_engine_subscriber_socket(&transport.ipc.message_proxy)?;

        let event_bus_sub_socket = EventBus::create_event_subscriber(&self.context, &[INPROC_APP_TOPIC])?;

        let mut items = [
            event_bus_sub_socket.as_poll_item(zmq::POLLIN),
            engine_subscriber_socket.as_poll_item(zmq::POLLIN),
        ];

        self.is_running = true;
        while self.is_running {
            // Poll both sockets
            if zmq::poll(&mut items, -1).is_ok() {
                // Check for event bus messages
                if items[0].is_readable() {
                    self.read_event_bus(&event_bus_sub_socket);
                }

                // Check for engine SUB messages (if running)
                if items[1].is_readable() && self.is_running {
                    self.forward_engine_message(&engine_subscriber_socket, &relay_publisher_socket);
                }
            }
        }

        debug!("Stopped Engine Message Proxy");

        Ok(())
    }

    fn create_relay_publisher_socket(&self, port: u32) -> Result<zmq::Socket> {
        let socket = self.context.socket(zmq::PUB)?;
        socket.set_linger(0)?;
        let address = format!("tcp://*:{}", port);
        match socket.bind(address.as_str()) {
            Ok(_) => debug!("Message Proxy bound to {}", address),
            Err(error) => {
                error!("Failed to bind PUB socket to {}: {}", address, error);
                process::exit(1);
            }
        }

        Ok(socket)
    }

    fn create_engine_subscriber_socket(&self, path: &str) -> Result<zmq::Socket> {
        let socket = self.context.socket(zmq::SUB)?;
        // Listen on all topics
        socket.set_subscribe(b"")?;
        socket.set_linger(0)?;
        let address = format!("ipc://{}", path);
        if let Err(error) = socket.bind(address.as_str()) {
            error!("Failed to bind engine SUB socket to {}: {}", address, error);
            process::exit(1);
        }

        // Make sure socket is accessible only to current user
        System::chmod(path, 0o700)?;

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

            } else {
                warn!("Got unknown event bus command: {}", event);
            }
        }
    }

    fn forward_engine_message(&self, engine_subscriber_socket: &zmq::Socket, relay_publisher_socket: &zmq::Socket) {
        let mut msg = zmq::Message::new();

        // Get message on subscriber socket
        if let Err(error) = engine_subscriber_socket.recv(&mut msg, 0) {
            error!("Failed to received message from engine message publisher: {}", error);

        } else {
            trace!("Got message from engine of length {}", msg.len());
            // Resend message on publisher socket
            if let Err(error) = relay_publisher_socket.send(msg, 0) {
                error!("Failed to send message to relay message subscriber: {}", error);
            }   
        }
    }

}
