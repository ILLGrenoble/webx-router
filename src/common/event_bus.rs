use crate::common::{Result};
use std::process;

static EVENT_BUS_SUB_ADDR: &str = "inproc://event-bus/subscriber";
static EVENT_BUS_PUB_ADDR: &str = "inproc://event-bus/publisher";

pub static INPROC_APP_TOPIC: &str = "app";
pub static INPROC_SESSION_TOPIC: &str = "session";

pub static APPLICATION_SHUTDOWN_COMMAND: &str = "app:shutdown";

pub struct EventBus {
    context: zmq::Context
}

impl EventBus {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context
        }
    }

    pub fn run(&self) -> Result<()> {
        // Create proxy subcriber
        let xsub_socket = self.create_proxy_subscriber(&self.context).unwrap();

        // Create proxy publisher
        let xpub_socket = self.create_proxy_publisher(&self.context).unwrap();

        let mut running = true;
        while running {
            let mut msg = zmq::Message::new();

            // Get next published event
            if let Err(error) = xsub_socket.recv(&mut msg, 0) {
                error!("Failed to receive event bus message: {}", error);

            } else {
                let event = msg.as_str().unwrap();

                // Check for shutdown
                if event == APPLICATION_SHUTDOWN_COMMAND {
                    running = false;
                }

                // Forward all events
                if let Err(error) = xpub_socket.send(msg, 0) {
                    error!("Failed to send message on event bus publisher: {}", error);
                }   
            }
        }

        info!("Stopped event bus");

        Ok(())
    }

    fn create_proxy_subscriber(&self, context: &zmq::Context) -> Result<zmq::Socket> {
        let socket = context.socket(zmq::SUB)?;
        socket.set_subscribe(b"")?;
        socket.set_linger(0)?;
        if let Err(error) = socket.bind(EVENT_BUS_SUB_ADDR) {
            error!("Failed to bind event bus XSUB to {}: {}", EVENT_BUS_SUB_ADDR, error);
            process::exit(1);
        }

        Ok(socket)
    }

    fn create_proxy_publisher(&self, context: &zmq::Context) -> Result<zmq::Socket> {
        let socket = context.socket(zmq::PUB)?;
        socket.set_linger(0)?;
        if let Err(error) = socket.bind(EVENT_BUS_PUB_ADDR) {
            error!("Failed to bind event bus XPUB to {}: {}", EVENT_BUS_PUB_ADDR, error);
            process::exit(1);
        }

        Ok(socket)
    }

    pub fn create_event_publisher(context: &zmq::Context) -> Result<zmq::Socket> {
        let socket = context.socket(zmq::PUB)?;
        socket.set_linger(0)?;

        if let Err(error) = socket.connect(EVENT_BUS_SUB_ADDR) {
            error!("Failed to connect inproc event publisher to {}: {}", EVENT_BUS_SUB_ADDR, error);
            process::exit(1);
        }

        Ok(socket)
    }

    pub fn create_event_subscriber(context: &zmq::Context, topics: &[&str]) -> Result<zmq::Socket> {
        let socket = context.socket(zmq::SUB)?;
        if topics.is_empty() {
            socket.set_subscribe(b"")?;
        } else {
            for topic in topics {
                socket.set_subscribe(topic.as_bytes())?;
            }
        }
        socket.set_linger(0)?;

        if let Err(error) = socket.connect(EVENT_BUS_PUB_ADDR) {
            error!("Failed to connect inproc event subscriber to {}: {}", EVENT_BUS_PUB_ADDR, error);
            process::exit(1);
        }

        Ok(socket)
    }    
}