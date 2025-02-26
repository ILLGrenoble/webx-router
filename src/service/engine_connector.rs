use crate::common::*;

pub struct EngineConnector {
    context: zmq::Context,
}

impl EngineConnector {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context,
        }
    }

    pub fn send_request(&self, path: &str, request: &str) -> Result<String> {
        // Create REQ socket
        let req_socket = self.create_req_socket(path)?;

        // Send requet message
        debug!("Sending WebX Engine request at {}", path);
        if let Err(error) = req_socket.send(request, 0) {
            error!("Failed to send request to {}: {}", path, error);
            return Err(RouterError::TransportError("Failed to send request to WebX Engine".to_string()));
        }

        trace!("Waiting for response from WebX Engine at {}", path);
        let mut response = zmq::Message::new();
        if let Err(error) = req_socket.recv(&mut response, 0) {
            error!("Failed to receive response from {}: {}", path, error);
            return Err(RouterError::TransportError("Failed to received response from WebX Engine".to_string()));
        }

        let message = response.as_str().unwrap();
        debug!("Received response {} from WebX Engine on {}", &message, &path);

        self.disconnect_req_socket(&req_socket, path);

        Ok(message.to_string())
    }

    fn create_req_socket(&self, path: &str) -> Result<zmq::Socket> {
        let socket = self.context.socket(zmq::REQ)?;
        socket.set_linger(0)?;
        socket.set_rcvtimeo(1000)?;

        let address = format!("ipc://{}", path);
        match socket.connect(address.as_str()) {
            Ok(_) => trace!("Engine Connector connected to {}", address),
            Err(error) => return Err(RouterError::TransportError(format!("Failed to connect REQ socket to {}: {}", address, error)))
        }

        Ok(socket)
    }

    fn disconnect_req_socket(&self, socket: &zmq::Socket, path: &str) {
        let address = format!("ipc://{}", path);
        match socket.disconnect(&address) {
            Ok(_) => trace!("Disconnected from Engine Connector socket at {}:", path),
            Err(error) => warn!("Failed to disconnect from Engine Connector socket at {}: {}", path, error)
        }
    }
}
