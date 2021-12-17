use crate::common::*;
use std::process;

pub struct RelayInstructionProxy {
    context: zmq::Context,
}

impl RelayInstructionProxy {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context,
        }
    }

    pub fn run(&self, settings: &Settings) -> Result<()> {
        let transport = &settings.transport;

        let relay_sub_socket = self.create_relay_sub_socket(transport.ports.collector)?;

        let engine_pub_socket = self.create_engine_pub_socket(&transport.ipc.instruction_proxy)?;

        let event_bus_sub_socket = EventBus::create_event_subscriber(&self.context, &[INPROC_APP_TOPIC])?;

        let mut is_running = true;
        while is_running {
            let mut msg = zmq::Message::new();

            let mut items = [
                event_bus_sub_socket.as_poll_item(zmq::POLLIN),
                relay_sub_socket.as_poll_item(zmq::POLLIN),
            ];

            // Poll both sockets
            if let Ok(_) = zmq::poll(&mut items, -1) {
                // Check for message_bus messages
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

                // Check for relay PUB messages (if running)
                if items[1].is_readable() && is_running {
                    // Get message from relay publisher
                    if let Err(error) = relay_sub_socket.recv(&mut msg, 0) {
                        error!("Failed to received message from relay publisher: {}", error);

                    } else {
                        trace!("Got message from relay of length {}", msg.len());
                        // Resend message on engine pub socket
                        if let Err(error) = engine_pub_socket.send(msg, 0) {
                            error!("Failed to send message to engine subscribers: {}", error);
                        }   
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

    fn create_engine_pub_socket(&self, path: &String) -> Result<zmq::Socket> {
        let socket = self.context.socket(zmq::PUB)?;
        socket.set_linger(0)?;
        let address = format!("ipc://{}", path);
        if let Err(error) = socket.bind(address.as_str()) {
            error!("Failed to bind engine PUB socket to {}: {}", address, error);
            process::exit(1);
        }

        // Make sure socket is accessible to all users
        User::change_file_permissions(path, "777")?;

        Ok(socket)
    }
}
