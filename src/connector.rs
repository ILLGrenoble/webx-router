use crate::publisher::Publisher;
use crate::inproc_communicator::{ProcessCommunicator, SHUTDOWN_COMMAND};
use std::thread;

static CONNECTOR_PORT: i32 = 5555;
static COLLECTOR_PORT: i32 = 5556;
static PUBLISHER_PORT: i32 = 5557;


pub struct Connector {
    context: zmq:: Context,
    rep_socket: Option<zmq::Socket>,
    inproc_sub_socket: Option<zmq::Socket>,
}

impl Connector {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context: context,
            rep_socket: None,
            inproc_sub_socket: None,
        }
    }

    pub fn init(&mut self) {
        // Create REP socket
        self.rep_socket = self.create_rep_socket();

        // Create shutdown SUB
        self.inproc_sub_socket = ProcessCommunicator::create_inproc_subscriber(&self.context);
    }

    pub fn run(&mut self) {
        // Create and run the publisher in separate thread
        let publisher_thread = self.create_publisher_thread(self.context.clone());

        let rep_socket = self.rep_socket.as_ref().unwrap();
        let inproc_sub_socket = self.inproc_sub_socket.as_ref().unwrap();

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
                        if is_running {
                            error!("Failed to receive shutdown message: {}", error);
                        }
                    } else {
                        if msg.as_str().unwrap() == SHUTDOWN_COMMAND {
                            is_running = false;
                        }
                    }
                }

                // Check for REQ-REP message (if running)
                if items[1].is_readable() && is_running {
                    if let Err(error) = rep_socket.recv(&mut msg, 0) {
                        error!("Failed to received message: {}", error);

                    } else {
                        let message_text = msg.as_str().unwrap();

                        // Check for comm message
                        if msg.len() == 4 && message_text == "comm" {
                            // Send response
                            if let Err(error) = rep_socket.send(format!("{},{}", PUBLISHER_PORT, COLLECTOR_PORT).as_str(), 0) {
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
        publisher_thread.join().unwrap();
    }

    fn create_rep_socket(&self) -> Option<zmq::Socket> {
        let socket = self.context.socket(zmq::REP).unwrap();
        socket.set_linger(0).unwrap();
        let address = format!("tcp://*:{}", CONNECTOR_PORT);
        if let Err(error) = socket.bind(address.as_str()) {
            error!("Failed to bind REP socket to {}: {}", address, error);
            return None;
        }

        Some(socket)
    }

    fn create_publisher_thread(&mut self, context: zmq::Context) -> thread::JoinHandle<()>{
        thread::spawn(move || {
            let mut publisher = Publisher::new(context);
            publisher.init();
            publisher.run();
        })
    }

}
