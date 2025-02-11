use crate::router::{EngineMessageProxy, RelayInstructionProxy, ClientConnector, SessionProxy};
use crate::common::*;

use std::thread;

pub struct Transport {
    context: zmq::Context,
}

impl Transport {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context,
        }
    }

    pub fn run(&self, settings: &mut Settings) -> Result<()> {
        let transport = &mut settings.transport;

        // Check for public/private keys in settings
        if transport.encryption.private.is_empty() || transport.encryption.public.is_empty() {
            let server_pair = zmq::CurveKeyPair::new()?;
            let public_key_string = zmq::z85_encode(&server_pair.public_key).unwrap();
            let secret_key_string = zmq::z85_encode(&server_pair.secret_key).unwrap();

            info!("Encyption keys not set in application config: generating new ones");
            transport.encryption.public = public_key_string;
            transport.encryption.private = secret_key_string;
        }

        // Create and run the engine message proxy in separate thread
        let engine_message_proxy_thread = self.create_engine_message_proxy_thread(self.context.clone(), settings);

        // Create and run the relay instruction proxy in separate thread
        let relay_instruction_proxy_thread = self.create_relay_instruction_proxy_thread(self.context.clone(), settings);

        // Create and run the session proxy in separate thread
        let session_proxy_thread = self.create_session_proxy_thread(self.context.clone(), settings);

        // Create and run the Client Connector in the current thread (blocking)
        if let Err(error) = ClientConnector::new(self.context.clone()).run(settings) {
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

    fn create_engine_message_proxy_thread(&self, context: zmq::Context, settings: &Settings) -> thread::JoinHandle<()>{
        thread::spawn({
            let settings = settings.clone();
            move || {
            if let Err(error) = EngineMessageProxy::new(context).run(&settings) {
                error!("Engine Message Proxy thread error: {}", error);
            }
        }})
    }

    fn create_relay_instruction_proxy_thread(&self, context: zmq::Context, settings: &Settings) -> thread::JoinHandle<()>{
        thread::spawn({
            let settings = settings.clone();
            move || {
            if let Err(error) = RelayInstructionProxy::new(context).run(&settings) {
                error!("Relay Instruction Proxy thread error: {}", error);
            }
        }})
    }

    fn create_session_proxy_thread(&self, context: zmq::Context, settings: &Settings) -> thread::JoinHandle<()>{
        thread::spawn({
            let settings = settings.clone();
            move || {
            if let Err(error) = SessionProxy::new(context).run(&settings) {
                error!("Session Proxy thread error: {}", error);
            }
        }})
    }

}
