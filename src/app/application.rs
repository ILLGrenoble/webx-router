use crate::common::{Settings, EventBus, APPLICATION_SHUTDOWN_COMMAND, Result};
use crate::router::Transport;

use std::thread;
use nix::unistd::User;

/// Represents the main application responsible for initializing and running the WebX Router.
pub struct Application {
}

impl Application {
    /// Creates a new instance of the `Application`.
    pub fn new() -> Self {
        Self {
        }
    }

    /// Runs the application by initializing components and starting the transport layer loop, awaiting requests from the WebX Relay.
    ///
    /// # Arguments
    /// * `settings` - Mutable reference to the application settings.
    ///
    /// # Returns
    /// * `Result<()>` - Indicates success or failure of the operation.
    pub fn run(&self, settings: &mut Settings, webx_user: User) -> Result<()> {
        info!("Starting WebX Router...");

        // Create ZMQ context
        let context = zmq::Context::new();
    
        // Create event bus
        let event_bus_thread = self.create_event_bus_thread(context.clone());
    
        // Create CTRL-C shutdown publisher
        self.create_shutdown_publisher(&context);
    
        // Create transport
        let transport = Transport::new(context);
    
        info!("WebX Router running");
        transport.run(settings, webx_user)?;
    
        // Join event bus thread
        event_bus_thread.join().unwrap();

        info!("WebX Router terminated");
        Ok(())
    }

    /// Creates a thread for the event bus and starts its execution.
    ///
    /// # Arguments
    /// * `context` - The ZeroMQ context used for communication.
    ///
    /// # Returns
    /// * `thread::JoinHandle<()>` - Handle to the spawned thread.
    fn create_event_bus_thread(&self, context: zmq::Context) -> thread::JoinHandle<()> {
        thread::spawn(move ||  {
            if let Err(error) = EventBus::new(context).run() {
                error!("Event Bus thread error: {}", error);
            }
        })
    }

    /// Sets up a shutdown publisher that listens for CTRL-C signals and sends a shutdown command on the event bus.
    ///
    /// # Arguments
    /// * `context` - Reference to the ZeroMQ context used for communication.
    fn create_shutdown_publisher(&self, context: &zmq::Context) {
        let socket = EventBus::create_event_publisher(context).unwrap();
        ctrlc::set_handler(move || {
            info!("Sending shutdown command");
            socket.send(APPLICATION_SHUTDOWN_COMMAND, 0).unwrap();

        }).expect("Error setting Ctrl-C handler");
    }
}

