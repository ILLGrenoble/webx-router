use crate::common::{EventBus, APPLICATION_SHUTDOWN_COMMAND, Result};
use crate::router::ClientConnector;
use std::thread;

pub struct Application {
}

impl Application {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn run(&self) -> Result<()> {
        info!("Starting WebX Router...");
    
        // Create ZMQ context
        let context = zmq::Context::new();
    
        // Create event bus
        let event_bus_thread = self.create_message_bus_thread(context.clone());
    
        // Create CTRL-C shutdown publisher
        self.create_shutdown_publisher(&context);
    
        // Create client connector
        let connector = ClientConnector::new(context);
    
        info!("WebX Router running");
        connector.run()?;
    
        // Join event bus thread
        event_bus_thread.join().unwrap()?;
    
        info!("WebX Router terminated");
        Ok(())
    }
    
    fn create_message_bus_thread(&self, context: zmq::Context) -> thread::JoinHandle<Result<()>> {
        thread::spawn(move || -> Result<()> {
            EventBus::new(context).run()
        })
    }
    
    fn create_shutdown_publisher(&self, context: &zmq::Context) {
        let socket = EventBus::create_event_publisher(context).unwrap();
        ctrlc::set_handler(move || {
            info!("Sending shutdown command");
            socket.send(APPLICATION_SHUTDOWN_COMMAND, 0).unwrap();
    
        }).expect("Error setting Ctrl-C handler");
    }    
}

