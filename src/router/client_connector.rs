use crate::router::{EngineMessageProxy, RelayInstructionProxy};
use crate::common::*;

use std::thread;

pub struct ClientConnector {
    context: zmq:: Context,
}

impl ClientConnector {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context: context,
        }
    }

    pub fn run(&self, settings: &TransportSettings) -> Result<()> {
        // Create and run the engine message proxy in separate thread
        let engine_message_proxy_thread = self.create_engine_message_proxy_thread(self.context.clone(), settings);

        // Create and run the relay instruction proxy in separate thread
        let relay_instruction_proxy_thread = self.create_relay_instruction_proxy_thread(self.context.clone(), settings);

        // Create REP socket
        let rep_socket = self.create_rep_socket(settings)?;

        // Create event bus SUB
        let event_bus_sub_socket = EventBus::create_event_subscriber(&self.context, &[INPROC_APP_TOPIC])?;

        let mut is_running = true;
        while is_running {
            let mut msg = zmq::Message::new();

            let mut items = [
                event_bus_sub_socket.as_poll_item(zmq::POLLIN),
                rep_socket.as_poll_item(zmq::POLLIN),
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
                            warn!("Got unknown event bus message: {}", event);
                        }
                    }
                }

                // Check for REQ-REP message (if running)
                if items[1].is_readable() && is_running {
                    if let Err(error) = rep_socket.recv(&mut msg, 0) {
                        error!("Failed to received message on relay req-rep: {}", error);

                    } else {
                        let message_text = msg.as_str().unwrap();

                        // Check for comm message
                        if msg.len() == 4 && message_text == "comm" {
                            // Send response
                            if let Err(error) = rep_socket.send(format!("{},{}", settings.ports.publisher, settings.ports.collector).as_str(), 0) {
                                error!("Failed to send comm message: {}", error);
                            }

                        } else {
                            // If send needed then send empty message
                            let empty_message = zmq::Message::new();
                            if let Err(error) = rep_socket.send(empty_message, 0) {
                                error!("Failed to send empty message: {}", error);
                            }
                        }
                    }
                }
            }
        }

        info!("Stopped Client Connector");

        // Join engine message proxy thread
        engine_message_proxy_thread.join().unwrap();

        // Join relay instruction proxy thread
        relay_instruction_proxy_thread.join().unwrap();

        Ok(())
    }

    fn create_rep_socket(&self, settings: &TransportSettings) -> Result<zmq::Socket> {
        let socket = self.context.socket(zmq::REP)?;
        socket.set_linger(0)?;

        let address = format!("tcp://*:{}", settings.ports.connector);
        match socket.bind(address.as_str()) {
            Ok(_) => info!("Client Connector bound to {}", address),
            Err(error) => return Err(RouterError::Transport(format!("Failed to bind REP socket to {}: {}", address, error)))
        }

        Ok(socket)
    }

    fn create_engine_message_proxy_thread(&self, context: zmq::Context, settings: &TransportSettings) -> thread::JoinHandle<()>{
        thread::spawn({
            let publisher_port = settings.ports.publisher;
            let message_proxy_addr = settings.ipc.message_proxy.clone();
            move || {
            if let Err(error) = EngineMessageProxy::new(context, publisher_port, message_proxy_addr).run() {
                error!("Engine Message Proxy thread error: {}", error);
            }
        }})
    }

    fn create_relay_instruction_proxy_thread(&self, context: zmq::Context, settings: &TransportSettings) -> thread::JoinHandle<()>{
        thread::spawn({
            let collector_port = settings.ports.collector;
            let instruction_proxy_addr = settings.ipc.instruction_proxy.clone();
            move || {
            if let Err(error) = RelayInstructionProxy::new(context, collector_port, instruction_proxy_addr).run() {
                error!("Relay Instruction Proxy thread error: {}", error);
            }
        }})
    }

}
