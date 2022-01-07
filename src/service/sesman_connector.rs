use crate::common::*;

pub struct SesmanConnector {
    context: zmq::Context,
    socket: Option<zmq::Socket>,
    ipc_path: String,
}

impl SesmanConnector {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context,
            socket: None,
            ipc_path: "".to_string(),
        }
    }

    pub fn open(&mut self, ipc_path: &str) -> Result<()> {
        match &self.socket {
            None => {
                let socket = self.create_req_socket(ipc_path)?;
                self.socket = Some(socket);
                self.ipc_path = ipc_path.to_string();
            },
            Some(_) => {}
        }

        Ok(())
    }

    pub fn close(&mut self) {
        match &self.socket {
            Some(socket) => {
                self.disconnect_req_socket(&socket, &self.ipc_path);
                self.socket = None;
                self.ipc_path = "".to_string();
            },
            None => {}
        }
    }

    pub fn get_authenticated_x11_session(&self, _username: &str, _password: &str) -> Result<()> {
        match &self.socket {
            Some(socket) => {
                // Send x11 session request
                debug!("Sending X11 session request");
                if let Err(error) = socket.send("", 0) {
                    error!("Failed to send X11 session request: {}", error);
                    return Err(RouterError::TransportError("Failed to send X11 session request".to_string()));
                }

                debug!("Waiting for X11 session response");
                let mut response = zmq::Message::new();
                if let Err(error) = socket.recv(&mut response, 0) {
                    error!("Failed to receive response to X11 session request: {}", error);
                    return Err(RouterError::TransportError("Failed to received X11 session request response".to_string()));
                }


                debug!("Received X11 session request response");

                Ok(())

            },
            None => {
                Err(RouterError::SessionError("Not connected to WebX Session Manager".to_string()))
            }
        }
    }

    fn create_req_socket(&self, path: &str) -> Result<zmq::Socket> {
        let socket = self.context.socket(zmq::REQ)?;
        socket.set_linger(0)?;
        socket.set_rcvtimeo(1000)?;

        let address = format!("ipc://{}", path);
        match socket.connect(address.as_str()) {
            Ok(_) => debug!("Sesman Connector connected to {}", address),
            Err(error) => return Err(RouterError::TransportError(format!("Failed to connect Sesman REQ socket to {}: {}", address, error)))
        }

        Ok(socket)
    }

    fn disconnect_req_socket(&self, socket: &zmq::Socket, path: &str) {
        let address = format!("ipc://{}", path);
        match socket.disconnect(&address) {
            Ok(_) => debug!("Disconnected from Sesman Connector socket at {}:", path),
            Err(error) => warn!("Failed to disconnect from Sesman Connector socket at {}: {}", path, error)
        }
    }
}
