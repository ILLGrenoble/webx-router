use crate::common::*;

/// Handles communication with the WebX Engine using ZeroMQ sockets.
pub struct EngineCommunicator {
    context: zmq::Context,
    req_socket: Option<zmq::Socket>,
    path: String
}

impl EngineCommunicator {
    /// Creates a new instance of the `EngineCommunicator`.
    ///
    /// # Arguments
    /// * `context` - The ZeroMQ context used for communication.
    /// * `path` - The IPC path to connect to the WebX Engine.
    ///
    /// # Returns
    /// * `EngineCommunicator` - A new instance of the communicator.
    pub fn new(context: zmq::Context, path: String) -> Self {
        Self {
            context,
            path: path,
            req_socket: None
        }
    }

    /// Closes the current request socket and disconnects from the engine.
    ///
    /// # Returns
    /// Nothing.
    pub fn close(&mut self) {
        self.disconnect_req_socket();
        self.req_socket = None;
    }

    /// Returns the IPC path this communicator is using.
    ///
    /// # Returns
    /// * `&str` - The IPC path as a string slice.
    pub fn path(&self) -> &str {
        &self.path
    }


    /// Resets the communicator: closes the current socket if it exists.
    pub fn reset(&mut self) {
        self.close();
    }

    /// Sends a request to the WebX Engine and waits for a response.
    ///
    /// # Arguments
    /// * `request` - The request string to send to the engine.
    ///
    /// # Returns
    /// * `Result<String>` - The response from the engine, or an error if communication fails.
    pub fn send_request(&mut self, request: &str) -> Result<String> {
        let req_socket = match self.req_socket {
            Some(ref mut req_socket) => req_socket,
            None => {
                let new_socket = self.create_req_socket()?;
                self.req_socket.insert(new_socket)
            }
        };

        // Send request message
        trace!("Sending WebX Engine request at {}", self.path);
        if let Err(error) = req_socket.send(request, 0) {
            error!("Failed to send request to {}: {}", self.path, error);
            return Err(RouterError::TransportError("Failed to send request to WebX Engine".to_string()));
        }

        trace!("Waiting for response from WebX Engine at {}", self.path);
        let mut response = zmq::Message::new();
        if let Err(error) = req_socket.recv(&mut response, 0) {
            error!("Failed to receive response from {}: {}", self.path, error);
            return Err(RouterError::TransportError("Failed to received response from WebX Engine".to_string()));
        }

        let message = response.as_str().unwrap();
        trace!("Received response {} from WebX Engine on {}", &message, &self.path);

        Ok(message.to_string())
    }

    /// Creates a ZeroMQ REQ socket and connects it to the specified path.
    ///
    /// # Returns
    /// * `Result<zmq::Socket>` - The created and connected socket or an error.
    fn create_req_socket(&self) -> Result<zmq::Socket> {
        let socket = self.context.socket(zmq::REQ)?;
        socket.set_linger(0)?;
        socket.set_rcvtimeo(1000)?;

        let address = format!("ipc://{}", self.path);
        match socket.connect(address.as_str()) {
            Ok(_) => trace!("Engine Connector connected to {}", self.path),
            Err(error) => return Err(RouterError::TransportError(format!("Failed to connect REQ socket to {}: {}", self.path, error)))
        }

        Ok(socket)
    }

    /// Disconnects a ZeroMQ REQ socket from the specified path.
    fn disconnect_req_socket(&self) {
        let address = format!("ipc://{}", self.path);
        if let Some(socket) = &self.req_socket {
            match socket.disconnect(&address) {
                Ok(_) => trace!("Disconnected from Engine Connector socket at {}:", self.path),
                Err(error) => warn!("Failed to disconnect from Engine Connector socket at {}: {}", self.path, error)
            }
        } 
    }
}