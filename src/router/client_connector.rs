use crate::router::{EngineMessageProxy, RelayInstructionProxy};
use crate::common::*;

use std::thread;

static RELAY_CONNECTOR_PORT: u32 = 5559;
static RELAY_PUBLISHER_PORT: u32 = 5557;
static RELAY_COLLECTOR_PORT: u32 = 5556;

static ENGINE_PUB_SUB_ADDR: &str = "ipc:///tmp/webx-router-engine-pub-sub.ipc";
static ENGINE_PULL_PUSH_ADDR: &str = "ipc:///tmp/webx-router-engine-pull-push.ipc";
// static ENGINE_PULL_PUSH_ADDR_FMT: &str = "ipc:///tmp/webx-router-engine-pull-push-{}.ipc";

pub struct ClientConnector {
    context: zmq:: Context,
}

impl ClientConnector {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context: context,
        }
    }

    pub fn run(&self) -> Result<()> {
        // Create and run the engine message proxy in separate thread
        let engine_message_proxy_thread = self.create_engine_message_proxy_thread(self.context.clone());

        // Create and run the relay instruction proxy in separate thread
        let relay_instruction_proxy_thread = self.create_relay_instruction_proxy_thread(self.context.clone());

        // Create REP socket
        let rep_socket = self.create_rep_socket()?;

        // Create event bus SUB
        let event_bus_sub_socket = EventBus::create_event_subscriber(&self.context, &[INPROC_APP_TOPIC])?;

        // Create event bus PUB
        let event_bus_pub_socket = EventBus::create_event_publisher(&self.context)?;

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
                            if let Err(error) = rep_socket.send(format!("{},{}", RELAY_PUBLISHER_PORT, RELAY_COLLECTOR_PORT).as_str(), 0) {
                                error!("Failed to send comm message: {}", error);
                            }

                            // Send event bus session message
                            if let Err(error) = event_bus_pub_socket.send(ENGINE_SESSION_START_COMMAND, 0) {
                                error!("Failed to send event bus session start message: {}", error);
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
        engine_message_proxy_thread.join().unwrap()?;

        // Join relay instruction proxy thread
        relay_instruction_proxy_thread.join().unwrap()?;

        Ok(())
    }

    fn create_rep_socket(&self) -> Result<zmq::Socket> {
        let socket = self.context.socket(zmq::REP)?;
        socket.set_linger(0)?;
        let address = format!("tcp://*:{}", RELAY_CONNECTOR_PORT);
        if let Err(error) = socket.bind(address.as_str()) {
            return Err(RouterError::Transport(format!("Failed to bind REP socket to {}: {}", address, error)));
        }

        Ok(socket)
    }

    fn create_engine_message_proxy_thread(&self, context: zmq::Context) -> thread::JoinHandle<Result<()>>{
        thread::spawn(move || {
            EngineMessageProxy::new(context, RELAY_PUBLISHER_PORT, ENGINE_PUB_SUB_ADDR.to_string()).run()
        })
    }

    fn create_relay_instruction_proxy_thread(&self, context: zmq::Context) -> thread::JoinHandle<Result<()>>{
        thread::spawn(move || {
            RelayInstructionProxy::new(context, RELAY_COLLECTOR_PORT, ENGINE_PULL_PUSH_ADDR.to_string()).run()
        })
    }

}
