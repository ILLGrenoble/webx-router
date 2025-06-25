use crate::common::{Result, RouterError, ProcessHandle};
use super::EngineCommunicator;

use std::fs;

/// Represents an WebX Engine process and its inter-process communication (IPC) channel.
pub struct Engine {
    /// The child process running the WebX Engine.
    process: ProcessHandle,
    session_id: String,
    communicator: EngineCommunicator,
}

impl Engine {
    /// Creates a new `Engine` instance.
    ///
    /// # Arguments
    ///
    /// * `process` - The child process running the WebX Engine.
    /// * `ipc` - The IPC channel identifier.
    ///
    /// # Returns
    ///
    /// A new instance of `Engine`.
    pub fn new(process: ProcessHandle, session_id: &str, context: zmq::Context, ipc: String) -> Self {
        Self {
            process,
            session_id: session_id.to_string(),
            communicator: EngineCommunicator::new(context, ipc),
        }
    }

    pub fn get_session_id(&self) -> &str {
        return &self.session_id;
    }

    /// Sends a request to a WebX Engine and retrieves the response.
    ///
    /// # Arguments
    /// * `session_id` - The ID of the session.
    /// * `context` - The ZeroMQ context.
    /// * `request` - The request string to send.
    ///
    /// # Returns
    /// The response from the session.
    pub fn send_request(&mut self, request: &str) -> Result<String> {
        self.communicator.send_request(request)
    }

    pub fn is_running(&self) -> Option<bool> {
        self.process.is_running()
    }

    pub fn close(&mut self) -> Result<()> {
        // Close the IPC channel
        self.communicator.close();
        
        debug!("Killing WebX Engine with pid: {}", self.process.pid());
        match self.process.kill() {
            Ok(_) => {
                // Delete the IPC socket file
                let _ = fs::remove_file(self.communicator.path());
            },
            Err(error) => {
                return Err(RouterError::SystemError(format!("Failed to kill WebX Engine with pid {}: {}", self.process.pid(), error)));
            }
        }
        
        Ok(())
    }

}
