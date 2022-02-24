use crate::common::*;
use std::process;
use std::ops::Deref;
use hex;

pub struct RelayInstructionProxy {
    context: zmq::Context,
    is_running: bool,
}

impl RelayInstructionProxy {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context,
            is_running: false,
        }
    }

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

        debug!("Stopped Relay Instruction Proxy");

        Ok(())
    }

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

    fn create_engine_pub_socket(&self, path: &str) -> Result<zmq::Socket> {
        let socket = self.context.socket(zmq::PUB)?;
        socket.set_linger(0)?;
        let address = format!("ipc://{}", path);
        if let Err(error) = socket.bind(address.as_str()) {
            error!("Failed to bind engine PUB socket to {}: {}", address, error);
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
