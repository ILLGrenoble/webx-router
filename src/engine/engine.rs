use crate::common::{Result, RouterError};
use super::EngineCommunicator;

use std::process::Child;
use signal_child::Signalable;
use std::fs;

/// Represents an WebX Engine process and its inter-process communication (IPC) channel.
pub struct Engine {
    /// The child process running the WebX Engine.
    process: Child,
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
    pub fn new(process: Child, context: zmq::Context, ipc: String) -> Self {
        Self {
            process,
            communicator: EngineCommunicator::new(context, ipc),
        }
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

    pub fn close(&mut self) -> Result<()> {
        // Close the IPC channel
        self.communicator.close();
        
        match self.process.interrupt() {
            Ok(_) => {
                if let Err(error) = self.process.wait() {
                    return Err(RouterError::SystemError(format!("Failed to wait for WebX Engine process to terminate: {}", error)));

                } else {
                    // Delete the IPC socket file
                    let _ = fs::remove_file(self.communicator.path());
                }
            },
            Err(error) => {
                return Err(RouterError::SystemError(format!("Failed to interrupt WebX Engine process with pid {}: {}", self.process.id(), error)));
            }
        }
        
        Ok(())
    }

}
