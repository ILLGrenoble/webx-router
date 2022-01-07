use crate::common::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "request", content = "content")]
enum Request {
    #[serde(rename = "login")]
    Login { username: String, password: String },
    #[serde(rename = "who")]
    Who,
    #[serde(rename = "logout")]
    Logout { id: u32 },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "response", content = "content")]
enum Response {
    #[serde(rename = "login")]
    Login { process_id: u32, display_id: String, uid: u32, username: String, xauthority_file_path: String },
}

pub struct SessionManagerResponse {
    pub username: String,
    pub display_id: String,
    pub xauthority_file_path: String,
}

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

    pub fn get_authenticated_x11_session(&self, username: &str, password: &str) -> Result<SessionManagerResponse> {
        match &self.socket {
            Some(socket) => {
                // Create the requet
                let request = Request::Login{username: username.to_string(), password: password.to_string()};
                let request_message = serde_json::to_string(&request)?;

                // Send x11 session request
                debug!("Sending X11 session request");
                if let Err(error) = socket.send(&request_message, 0) {
                    error!("Failed to send X11 session request: {}", error);
                    return Err(RouterError::TransportError("Failed to send X11 session request".to_string()));
                }

                debug!("Waiting for X11 session response");
                let mut response = zmq::Message::new();
                if let Err(error) = socket.recv(&mut response, 0) {
                    error!("Failed to receive response to X11 session request: {}", error);
                    return Err(RouterError::TransportError("Failed to receive X11 session request response".to_string()));
                }

                let response_message = response.as_str().unwrap();
                debug!("Received X11 session request response: {}", &response_message);

                match serde_json::from_str::<Response>(&response_message) {
                    Ok(response) => match response {
                        Response::Login { username, display_id, xauthority_file_path, .. } => {
                            Ok(SessionManagerResponse {
                                username,
                                display_id,
                                xauthority_file_path,
                            })
                        }
                    },
                    Err(error) => {
                        error!("Failed to unserialise WebX Session Manager response: {}", error);
                        Err(RouterError::SessionError("Failed to unserialise WebX Session Manager response".to_string()))
                    },
                }
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
