use std::process::Command;
use std::sync::Arc;

use shared_child::SharedChild;

use crate::common::{Result, RouterError};

/// The `ProcessHandle` struct represents a handle to a linux process managed by the WebX Session Manager.
#[derive(Clone)]
pub struct ProcessHandle {
    process: Arc<SharedChild>,
}

impl ProcessHandle {
    /// Creates a new `ProcessHandle` by spawning a process using the provided command.
    ///
    /// # Arguments
    /// * `command` - The command to execute.
    ///
    /// # Returns
    /// A `Result` containing the `ProcessHandle` or an `ApplicationError` if the process could not be spawned.
    pub fn new(command: &mut Command) -> Result<ProcessHandle> {
        Ok(ProcessHandle {
            process: Arc::new(SharedChild::spawn(command)?),
        })
    }

    /// Kills the process associated with this handle.
    ///
    /// # Returns
    /// A `Result` indicating success or an `ApplicationError` if the process could not be killed.
    pub fn kill(&self) -> Result<()> {
        return match self.process.kill() {
            Ok(_) => Ok(()),
            Err(error) => Err(RouterError::IoError(error))
        }
    }

    /// Returns the process ID (PID) of the process.
    pub fn pid(&self) -> u32 {
        self.process.id()
    }

    pub fn is_running(&self) -> Option<bool> {
        let terminate_result = self.process.try_wait();
        match terminate_result {
            Ok(expected_status) => match expected_status {
                // Process already exited. Terminate was successful.
                Some(_status) => Some(false),
                None => Some(true)
            },
            Err(error) => {
                warn!("Failed to wait for process [pid={}]. Error: {}", self.process.id(), error);
                None
            }
        }
    }
}