use crate::common::{Settings, Result, EventBus, APPLICATION_SHUTDOWN_COMMAND};
use crate::router::Transport;

use std::thread;
use signal_hook::iterator::Signals;
use libc::{SIGINT, SIGQUIT, SIGTERM};

/// Represents the main application responsible for initializing and running the WebX Router.
pub struct Server {
}

impl Server {
    /// Creates a new instance of the `Server`.
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
    pub fn run(&self, settings: Settings) -> Result<()> {
        info!("Starting WebX Router...");

        // Create ZMQ context
        let context = zmq::Context::new();
    
        // Create event bus
        let event_bus_thread = self.create_event_bus_thread(context.clone());
    
        // Create shutdown publisher to listen to signals
        self.create_shutdown_publisher(&context);
     
        // Create transport
        let mut transport = Transport::new(context, settings);
        
        // Run transport blocking
        info!("WebX Router running");
        transport.run()?;

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
        thread::spawn(move ||  {

            // Set up signal handling
            let mut signals = Signals::new(&[SIGTERM, SIGINT, SIGQUIT])
                .expect("Signals::new() failed");

            // Wait for a signal. This will block until a signal is received
            signals.forever().next();

            info!("Termination signal received. Shutting down WebX Router...");
            socket.send(APPLICATION_SHUTDOWN_COMMAND, 0).unwrap();
        });
    }
}

