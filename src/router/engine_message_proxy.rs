use crate::common::*;
use crate::fs;
use std::process;

/// Handles the forwarding of messages from the engines to the relay.
pub struct EngineMessageProxy {
    context: zmq::Context,
    is_running: bool,
}

impl EngineMessageProxy {
    /// Creates a new instance of the `EngineMessageProxy`.
    ///
    /// # Arguments
    /// * `context` - The ZeroMQ context used for communication.
    pub fn new(context: zmq::Context) -> Self {
        Self {
            context,
            is_running: false,
        }
    }

    /// Runs the engine message proxy, forwarding messages between components.
    ///
    /// # Arguments
    /// * `settings` - Reference to the application settings.
    ///
    /// # Returns
    /// * `Result<()>` - Indicates success or failure of the operation.
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

    /// Creates a ZeroMQ PUB socket for publishing messages to the relay.
    ///
    /// # Arguments
    /// * `port` - The port to bind the socket to.
    ///
    /// # Returns
    /// * `Result<zmq::Socket>` - The created and bound socket or an error.
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

    /// Creates a ZeroMQ SUB socket for subscribing to engine messages.
    ///
    /// # Arguments
    /// * `path` - The IPC path to bind the socket to.
    ///
    /// # Returns
    /// * `Result<zmq::Socket>` - The created and bound socket or an error.
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

        // Make sure the socket is owned by the 'webx' user
        match System::get_user("webx") {
            Some(user) => {
                // Change ownership of the IPC socket to 'webx' user
                fs::chown(path, user.uid.as_raw(), user.gid.as_raw())?;

                // Make sure socket is accessible only to current user
                fs::chmod(path, 0o700)?;

                Ok(socket)
            },
            None => {
                error!("Cannot created engine PUB socket, user 'webx' not found");
                process::exit(1);
            }
        }
    }

    /// Reads messages from the event bus and handles shutdown commands.
    ///
    /// # Arguments
    /// * `event_bus_sub_socket` - The ZeroMQ socket subscribed to the event bus.
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

    /// Forwards messages from engines to the relay.
    ///
    /// # Arguments
    /// * `engine_subscriber_socket` - The ZeroMQ socket receiving engine messages.
    /// * `relay_publisher_socket` - The ZeroMQ socket publishing messages to the relay.
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
