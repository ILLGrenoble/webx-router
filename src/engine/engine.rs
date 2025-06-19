use std::process::Child;

/// Represents an WebX Engine process and its inter-process communication (IPC) channel.
pub struct Engine {
    /// The child process running the WebX Engine.
    process: Child,
    /// The IPC channel identifier for communication.
    ipc: String,
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
    pub fn new(process: Child, ipc: String) -> Self {
        Self {
            process,
            ipc,
        }
    }

    /// Provides mutable access to the managed child process (and the running WebX Engine).
    ///
    /// # Returns
    ///
    /// A mutable reference to the child process.
    pub fn process(&mut self) -> &mut Child {
        return &mut self.process;
    }

    /// Retrieves the IPC channel identifier.
    ///
    /// # Returns
    ///
    /// A string slice representing the IPC channel identifier.
    pub fn ipc(&self) -> &str {
        return &self.ipc;
    }
}
