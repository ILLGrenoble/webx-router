use crate::common::*;

pub struct SessionConnector {
    context: zmq::Context,
}

impl SessionConnector {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context,
        }
    }

    pub fn validate_connection(&self, path: &str) -> Result<()> {
        // Create REQ socket
        let req_socket = self.create_req_socket(path)?;

        // Send ping message
        debug!("Pinging WebX Engine at {}", path);
        if let Err(error) = req_socket.send("ping", 0) {
            error!("Failed to send ping command to {}: {}", path, error);
            return Err(RouterError::TransportError("Failed to send ping request".to_string()));
        }

        debug!("Waiting for pong on WebX Engine at {}", path);
        let mut response = zmq::Message::new();
        if let Err(error) = req_socket.recv(&mut response, 0) {
            error!("Failed to receive response to ping on {}: {}", path, error);
            return Err(RouterError::TransportError("Failed to received ping response".to_string()));
        }

        let message = response.as_str().unwrap();
        if message != "pong" {
            error!("Received non-pong response from {}: {}", path, message);
            return Err(RouterError::SessionError("Receivec non-pong message".to_string()));
        }

        debug!("Received pong response from {}", path);

        req_socket.disconnect(path)?;

        Ok(())
    }

    fn create_req_socket(&self, path: &str) -> Result<zmq::Socket> {
        let socket = self.context.socket(zmq::REQ)?;
        socket.set_linger(0)?;

        let address = format!("ipc://{}", path);
        match socket.connect(address.as_str()) {
            Ok(_) => debug!("Session Connector connected to {}", address),
            Err(error) => return Err(RouterError::TransportError(format!("Failed to connect REQ socket to {}: {}", address, error)))
        }

        Ok(socket)
    }

}
