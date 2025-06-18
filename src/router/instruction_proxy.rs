use crate::common::*;
use crate::fs;
use std::process;
use std::ops::Deref;
use hex;

/// Handles the forwarding of instructions from the relay to the engines.
pub struct InstructionProxy {
    context: zmq::Context,
    is_running: bool,
}

impl InstructionProxy {
    /// Creates a new instance of the `InstructionProxy`.
    ///
    /// # Arguments
    /// * `context` - The ZeroMQ context used for communication.
    pub fn new(context: zmq::Context) -> Self {
        Self {
            context,
            is_running: false,
        }
    }

    /// Runs the relay instruction proxy, forwarding messages between components.
    ///
    /// # Arguments
    /// * `settings` - Reference to the application settings.
    ///
    /// # Returns
    /// * `Result<()>` - Indicates success or failure of the operation.
    pub fn run(&mut self, settings: &Settings) -> Result<()> {
        let transport = &settings.transport;

        let relay_sub_socket = self.create_relay_sub_socket(transport.ports.collector)?;

        let engine_pub_socket = self.create_engine_pub_socket(&transport.ipc.instruction_proxy)?;

        let event_bus_sub_socket = EventBus::create_event_subscriber(&self.context, &[INPROC_APP_TOPIC])?;

        let event_bus_pub_socket = EventBus::create_event_publisher(&self.context)?;

        let mut items = [
            event_bus_sub_socket.as_poll_item(zmq::POLLIN),
            relay_sub_socket.as_poll_item(zmq::POLLIN),
        ];

        self.is_running = true;
        while self.is_running {
            // Poll both sockets
            if zmq::poll(&mut items, -1).is_ok() {
                // Check for message_bus messages
                if items[0].is_readable() {
                    self.read_event_bus(&event_bus_sub_socket);
                }

                // Check for relay PUB messages (if running)
                if items[1].is_readable() && self.is_running {
                    match self.forward_relay_instruction(&relay_sub_socket, &engine_pub_socket) {
                        // Send session id on inproc message queue, to be used by session_proxy
                        Some(session_id) => {
                            let session_message = format!("{}:{}", INPROC_SESSION_TOPIC, session_id);
                            event_bus_pub_socket.send(&session_message, 0).unwrap();
                        },
                        None => {}
                    }
                }
            }
        }

        debug!("Stopped Instruction Proxy");

        Ok(())
    }

    /// Creates a ZeroMQ SUB socket for receiving relay instructions.
    ///
    /// # Arguments
    /// * `port` - The port to bind the socket to.
    ///
    /// # Returns
    /// * `Result<zmq::Socket>` - The created and bound socket or an error.
    fn create_relay_sub_socket(&self, port: u32) -> Result<zmq::Socket> {
        let socket = self.context.socket(zmq::SUB)?;
        // Listen on all topics
        socket.set_subscribe(b"")?;
        socket.set_linger(0)?;
        let address = format!("tcp://*:{}", port);

        match socket.bind(address.as_str()) {
            Ok(_) => debug!("Instruction Proxy bound to {}", address),
            Err(error) => {
                error!("Failed to bind relay SUB socket to {}: {}", address, error);
                process::exit(1);
            }
        }

        Ok(socket)
    }

    /// Creates a ZeroMQ PUB socket for sending instructions to the engine.
    ///
    /// # Arguments
    /// * `path` - The IPC path to bind the socket to.
    ///
    /// # Returns
    /// * `Result<zmq::Socket>` - The created and bound socket or an error.
    fn create_engine_pub_socket(&self, path: &str) -> Result<zmq::Socket> {
        let socket = self.context.socket(zmq::PUB)?;
        socket.set_linger(0)?;
        let address = format!("ipc://{}", path);
        if let Err(error) = socket.bind(address.as_str()) {
            error!("Failed to bind engine PUB socket to {}: {}", address, error);
            process::exit(1);
        }

        // Make sure the socket is owned by the 'webx' user
        match System::get_user("webx") {
            Some(user) => {
                // Change ownership of the socket to 'webx' user
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

    /// Forwards relay instructions to the engines and extracts session ID (to update usage times for the session).
    ///
    /// # Arguments
    /// * `relay_sub_socket` - The ZeroMQ socket receiving relay instructions.
    /// * `engine_pub_socket` - The ZeroMQ socket publishing instructions to the engine.
    ///
    /// # Returns
    /// * `Option<String>` - The session ID if available.
    fn forward_relay_instruction(&self, relay_sub_socket: &zmq::Socket, engine_pub_socket: &zmq::Socket) -> Option<String> {
        let mut msg = zmq::Message::new();
        let mut session_id_option = None;

        // Get message from relay publisher
        if let Err(error) = relay_sub_socket.recv(&mut msg, 0) {
            error!("Failed to received instruction from relay publisher: {}", error);

        } else {
            trace!("Got instruction from relay of length {}", msg.len());

            // Get session_id from the msg
            let raw_session_id = msg.deref();
            let session_id = hex::encode(&raw_session_id[0 .. 16]);
            session_id_option = Some(session_id);

            // Resend message on engine pub socket
            if let Err(error) = engine_pub_socket.send(msg, 0) {
                error!("Failed to send instruction to engine subscribers: {}", error);
            }   
        }

        session_id_option
    }
}
