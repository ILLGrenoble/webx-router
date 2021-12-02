static MESSAGE_BUS_SUB_ADDR: &str = "inproc://message-bus/subscriber";
static MESSAGE_BUS_PUB_ADDR: &str = "inproc://message-bus/publisher";

pub static INPROC_APP_TOPIC: &str = "APP";
pub static INPROC_SESSION_TOPIC: &str = "SESSION";

pub static APPLICATION_SHUTDOWN_COMMAND: &str = "APP:SHUTDOWN";
pub static ENGINE_SESSION_START_COMMAND: &str = "SESSION:START";
pub static ENGINE_SESSION_END_COMMAND: &str = "SESSION:END";


pub struct MessageBus {
    context: zmq::Context
}

impl MessageBus {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context: context
        }
    }

    pub fn run(&self) {
        // Create proxy subcriber
        let xsub_socket = self.create_proxy_subscriber(&self.context).unwrap();

        // Create proxy publisher
        let xpub_socket = self.create_proxy_publisher(&self.context).unwrap();

        let mut running = true;
        while running {
            let mut msg = zmq::Message::new();

            // Get next published message
            if let Err(error) = xsub_socket.recv(&mut msg, 0) {
                error!("Failed to receive message bus message: {}", error);

            } else {
                let message = msg.as_str().unwrap();

                // Check for shutdown
                if message == APPLICATION_SHUTDOWN_COMMAND {
                    running = false;
                }

                // Forward all messages
                if let Err(error) = xpub_socket.send(msg, 0) {
                    error!("Failed to send message on message bus publisher: {}", error);
                }   
            }
        }

        info!("Stopped message bus");
    }

    fn create_proxy_subscriber(&self, context: &zmq::Context) -> Option<zmq::Socket> {
        let socket = context.socket(zmq::SUB).unwrap();
        socket.set_subscribe(b"").unwrap();
        socket.set_linger(0).unwrap();
        if let Err(error) = socket.bind(MESSAGE_BUS_SUB_ADDR) {
            error!("Failed to bind message bus XSUB to {}: {}", MESSAGE_BUS_SUB_ADDR, error);
            return None;
        }

        Some(socket)
    }

    fn create_proxy_publisher(&self, context: &zmq::Context) -> Option<zmq::Socket> {
        let socket = context.socket(zmq::PUB).unwrap();
        socket.set_linger(0).unwrap();
        if let Err(error) = socket.bind(MESSAGE_BUS_PUB_ADDR) {
            error!("Failed to bind message bus XPUB to {}: {}", MESSAGE_BUS_PUB_ADDR, error);
            return None;
        }

        Some(socket)
    }

    pub fn create_message_publisher(context: &zmq::Context) -> Option<zmq::Socket> {
        let socket = context.socket(zmq::PUB).unwrap();
        socket.set_linger(0).unwrap();
        if let Err(error) = socket.connect(MESSAGE_BUS_SUB_ADDR) {
            error!("Failed to connect inproc pub_sub to {}: {}", MESSAGE_BUS_SUB_ADDR, error);
            return None;
        }

        Some(socket)
    }

    pub fn create_message_subscriber(context: &zmq::Context, topics: &[&str]) -> Option<zmq::Socket> {
        let socket = context.socket(zmq::SUB).unwrap();
        if topics.is_empty() {
            socket.set_subscribe(b"").unwrap();
        } else {
            for topic in topics {
                socket.set_subscribe(topic.as_bytes()).unwrap();
            }
        }
        socket.set_linger(0).unwrap();

        if let Err(error) = socket.connect(MESSAGE_BUS_PUB_ADDR) {
            error!("Failed to connect inproc SUB socket to {}: {}", MESSAGE_BUS_PUB_ADDR, error);
            return None;
        }

        Some(socket)
    }    
}