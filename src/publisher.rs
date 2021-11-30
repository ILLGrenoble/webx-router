use crate::inproc_communicator::{ProcessCommunicator, SHUTDOWN_COMMAND};

pub struct Publisher {
    context: zmq::Context,
    inproc_sub_socket: Option<zmq::Socket>,
}

impl Publisher {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context: context,
            inproc_sub_socket: None
        }
    }

    pub fn init(&mut self) {
        self.inproc_sub_socket = ProcessCommunicator::create_inproc_subscriber(&self.context);
    }

    pub fn run(&self) {
        let inproc_sub_socket = self.inproc_sub_socket.as_ref().unwrap();

        let mut is_running = true;
        while is_running {
            let mut msg = zmq::Message::new();

            let mut items = [
                inproc_sub_socket.as_poll_item(zmq::POLLIN),
            ];

            // Poll both sockets
            zmq::poll(&mut items, -1).unwrap();

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
        }

        info!("Stopped client message publisher");
    }
}
