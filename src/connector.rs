use crate::pub_sub_proxy::PubSubProxy;
use crate::pull_push_proxy::PullPushProxy;
use crate::process_communicator::{ProcessCommunicator, SHUTDOWN_COMMAND, RELAY_CONNECTOR_PORT, RELAY_PUBLISHER_PORT, RELAY_COLLECTOR_PORT};
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

        // Create shutdown SUB
        let inproc_sub_socket = ProcessCommunicator::create_inproc_subscriber(&self.context).unwrap();

        let mut is_running = true;
        while is_running {
            let mut msg = zmq::Message::new();

            let mut items = [
                inproc_sub_socket.as_poll_item(zmq::POLLIN),
                rep_socket.as_poll_item(zmq::POLLIN),
            ];

            // Poll both sockets
            if let Ok(_) = zmq::poll(&mut items, -1) {
                // Check for shutdown message
                if items[0].is_readable() {
                    if let Err(error) = inproc_sub_socket.recv(&mut msg, 0) {
                        error!("Failed to receive shutdown message: {}", error);

                    } else {
                        if msg.as_str().unwrap() == SHUTDOWN_COMMAND {
                            is_running = false;
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
