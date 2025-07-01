use crate::router::{MessageProxy, InstructionProxy, ClientConnector, SessionProxy};
use crate::common::{Settings, Result};
use std::thread;

/// Manages the transport layer of the WebX Router, including proxies and connectors.
pub struct Transport {
    context: zmq::Context,
    settings: Settings,
}

impl Transport {
    /// Creates a new instance of the `Transport`.
    ///
    /// # Arguments
    /// * `context` - The ZeroMQ context used for communication.
    pub fn new(context: zmq::Context, settings: Settings) -> Self {
        Self {
            context,
            settings,
        }
    }

    /// Runs the transport layer by initializing and managing its components.
    ///
    /// # Arguments
    /// * `settings` - Mutable reference to the application settings.
    ///
    /// # Returns
    /// * `Result<()>` - Indicates success or failure of the operation.
    pub fn run(&mut self) -> Result<()> {
        // Generate encryption keys
        let server_pair = zmq::CurveKeyPair::new()?;
        let public_key = zmq::z85_encode(&server_pair.public_key).unwrap();
        let secret_key = zmq::z85_encode(&server_pair.secret_key).unwrap();

        // Create and run the engine message proxy in separate thread
        let engine_message_proxy_thread = self.create_engine_message_proxy_thread(self.context.clone(), &self.settings);

        // Create and run the relay instruction proxy in separate thread
        let relay_instruction_proxy_thread = self.create_relay_instruction_proxy_thread(self.context.clone(), &self.settings);

        // Create and run the session proxy in separate thread
        let session_proxy_thread = self.create_session_proxy_thread(self.context.clone(), &self.settings, &secret_key);

        // Create and run the Client Connector in the current thread (blocking)
        if let Err(error) = ClientConnector::new(self.context.clone()).run(&self.settings, &public_key) {
            error!("Error while running Client Connector: {}", error);
        }

        // Join engine message proxy thread
        engine_message_proxy_thread.join().unwrap();

        // Join relay instruction proxy thread
        relay_instruction_proxy_thread.join().unwrap();

        // Join relay instruction proxy thread
        session_proxy_thread.join().unwrap();

        Ok(())
    }

    /// Creates and starts the engine message proxy in a separate thread.
    ///
    /// # Arguments
    /// * `context` - The ZeroMQ context used for communication.
    /// * `settings` - Reference to the application settings.
    ///
    /// # Returns
    /// * `thread::JoinHandle<()>` - Handle to the spawned thread.
    fn create_engine_message_proxy_thread(&self, context: zmq::Context, settings: &Settings) -> thread::JoinHandle<()> {
        thread::spawn({
            let settings = settings.clone();
            move || {
            if let Err(error) = MessageProxy::new(context).run(&settings) {
                error!("Message Proxy thread error: {}", error);
            }
        }})
    }

    /// Creates and starts the relay instruction proxy in a separate thread.
    ///
    /// # Arguments
    /// * `context` - The ZeroMQ context used for communication.
    /// * `settings` - Reference to the application settings.
    ///
    /// # Returns
    /// * `thread::JoinHandle<()>` - Handle to the spawned thread.
    fn create_relay_instruction_proxy_thread(&self, context: zmq::Context, settings: &Settings) -> thread::JoinHandle<()> {
        thread::spawn({
            let settings = settings.clone();
            move || {
            if let Err(error) = InstructionProxy::new(context).run(&settings) {
                error!("Instruction Proxy thread error: {}", error);
            }
        }})
    }

    /// Creates and starts the session proxy in a separate thread.
    ///
    /// # Arguments
    /// * `context` - The ZeroMQ context used for communication.
    /// * `settings` - Reference to the application settings.
    ///
    /// # Returns
    /// * `thread::JoinHandle<()>` - Handle to the spawned thread.
    fn create_session_proxy_thread(&self, context: zmq::Context, settings: &Settings, secret_key: &str) -> thread::JoinHandle<()> {
        thread::spawn({
            let settings = settings.clone();
            let secret_key = secret_key.to_string();
            move || {
            if let Err(error) = SessionProxy::new(context, &settings.sesman).run(&settings, &secret_key) {
                error!("Session Proxy thread error: {}", error);
            }
        }})
    }

}
