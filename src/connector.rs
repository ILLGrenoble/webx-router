use crate::pub_sub_proxy::PubSubProxy;
use crate::pull_push_proxy::PullPushProxy;
use crate::message_bus::*;
use crate::common::*;

use std::thread;

pub struct Connector {
    context: zmq:: Context,
}

impl Connector {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context: context,
        }
    }

    pub fn run(&self) {
        // Create and run the pub-sub proxy in separate thread
        let pub_sub_proxy_thread = self.create_pub_sub_proxy_thread(self.context.clone());

        // Create and run the pull-push proxy in separate thread
        let pull_push_proxy_thread = self.create_pull_push_proxy_thread(self.context.clone());

        // Create REP socket
        let rep_socket = self.create_rep_socket().unwrap();

        // Create message bus SUB
        let message_bus_sub_socket = MessageBus::create_message_subscriber(&self.context, &[INPROC_APP_TOPIC]).unwrap();

        // Create message bus PUB
        let message_bus_pub_socket = MessageBus::create_message_publisher(&self.context).unwrap();

        let mut is_running = true;
        while is_running {
            let mut msg = zmq::Message::new();

            let mut items = [
                message_bus_sub_socket.as_poll_item(zmq::POLLIN),
                rep_socket.as_poll_item(zmq::POLLIN),
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
                        } else {
                            warn!("Got unknown message_bus command: {}", message_bus_message);
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

                            // Send message_bus session message
                            if let Err(error) = message_bus_pub_socket.send(ENGINE_SESSION_START_COMMAND, 0) {
                                error!("Failed to send message_bus session start message: {}", error);
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

        info!("Stopped client connector");

        // Join pub-sub proxy thread
        pub_sub_proxy_thread.join().unwrap();

        // Join pull-push proxy thread
        pull_push_proxy_thread.join().unwrap();
    }

    fn create_rep_socket(&self) -> Option<zmq::Socket> {
        let socket = self.context.socket(zmq::REP).unwrap();
        socket.set_linger(0).unwrap();
        let address = format!("tcp://*:{}", RELAY_CONNECTOR_PORT);
        if let Err(error) = socket.bind(address.as_str()) {
            error!("Failed to bind REP socket to {}: {}", address, error);
            return None;
        }

        Some(socket)
    }

    fn create_pub_sub_proxy_thread(&self, context: zmq::Context) -> thread::JoinHandle<()>{
        thread::spawn(move || {
            PubSubProxy::new(context).run();
        })
    }

    fn create_pull_push_proxy_thread(&self, context: zmq::Context) -> thread::JoinHandle<()>{
        thread::spawn(move || {
            PullPushProxy::new(context).run();
        })
    }

}
