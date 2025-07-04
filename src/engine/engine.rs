use crate::common::{Result, RouterError, ProcessHandle};
use super::EngineCommunicator;

use std::fs;

/// Represents a WebX Engine process and its inter-process communication (IPC) channel.
pub struct Engine {
    /// The child process running the WebX Engine.
    process: ProcessHandle,
    /// The session ID associated with this engine.
    session_id: String,
    /// The communicator used for IPC with the engine.
    communicator: EngineCommunicator,
}

impl Engine {
    /// Creates a new `Engine` instance.
    ///
    /// # Arguments
    /// * `process` - The child process running the WebX Engine.
    /// * `session_id` - The session ID associated with this engine.
    /// * `context` - The ZeroMQ context for communication.
    /// * `ipc` - The IPC channel identifier (path).
    ///
    /// # Returns
    /// * `Engine` - A new instance of `Engine`.
    pub fn new(process: ProcessHandle, session_id: &str, context: zmq::Context, ipc: String) -> Self {
        Self {
            process,
            session_id: session_id.to_string(),
            communicator: EngineCommunicator::new(context, ipc),
        }
    }

    /// Returns the session ID associated with this engine.
    ///
    /// # Returns
    /// * `&str` - The session ID.
    pub fn session_id(&self) -> &str {
        return &self.session_id;
    }

    /// Sends a request to the WebX Engine and retrieves the response.
    ///
    /// # Arguments
    /// * `request` - The request string to send.
    ///
    /// # Returns
    /// * `Result<String>` - The response from the engine, or an error if communication fails.
    pub fn send_request(&mut self, request: &str) -> Result<String> {
        self.communicator.send_request(request).map_err(|error| {
            debug!("Request failed to WebX Engine. Recreating the socket as it may have been created prematurely");
            self.communicator.reset();
            error
        }) 
    }

    /// Checks if the engine process is still running.
    ///
    /// # Returns
    /// * `Option<bool>` - Some(true) if running, Some(false) if not, or None if status cannot be determined.
    pub fn is_running(&self) -> Option<bool> {
        self.process.is_running()
    }

    /// Closes the engine process and its IPC channel, and removes the IPC socket file.
    ///
    /// # Returns
    /// * `Result<()>` - Ok if the engine was closed successfully, Err otherwise.
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