use crate::common::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "request", content = "content")]
enum SessionManagerRequest {
    #[serde(rename = "login")]
    Login { username: String, password: String, width: u32, height: u32 },
    
    #[serde(rename = "who")]
    Who,

    #[serde(rename = "logout")]
    Logout { id: String },
}

#[derive(Serialize, Deserialize)]
struct SessionManagerSession {
    id: String,
    username: String,
    uid: u32,
    display_id: String,
    xorg_process_id: u32,
    window_manager_process_id: u32,
    xauthority_file_path: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "response", content = "content")]
enum SessionManagerResponse {
    #[serde(rename = "login")]
    Login(SessionManagerSession),

    #[serde(rename = "who")]
    Who { sessions: Vec<SessionManagerSession> },

    #[serde(rename = "error")]
    Error { message: String },

    #[serde(rename = "logout")]
    Logout
}

pub struct SesmanConnector {
    context: zmq::Context,
}

impl SesmanConnector {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            context,
        }
    }

    pub fn get_authenticated_x11_session(&self, username: &str, password: &str, width: u32, height: u32, ipc_path: &str) -> Result<X11Session> {
        let socket = self.create_req_socket(ipc_path)?;

        let response = self.handle_sesman_login_request(username, password, width, height, &socket);

        self.disconnect_req_socket(&socket, ipc_path);

        response
    }

    pub fn logout(&self, session_id: &str, ipc_path: &str) -> Result<()> {
        let socket = self.create_req_socket(ipc_path)?;

        let response = self.handle_sesman_logout_request(session_id, &socket);

        self.disconnect_req_socket(&socket, ipc_path);

        response
    }

    fn handle_sesman_login_request(&self, username: &str, password: &str, width: u32, height: u32, socket: &zmq::Socket) -> Result<X11Session> {
        // Create the request
        let request = SessionManagerRequest::Login{username: username.to_string(), password: password.to_string(), width, height};
        let request_message = serde_json::to_string(&request)?;

        // Send x11 session request
        debug!("Sending X11 session login request");
        if let Err(error) = socket.send(&request_message, 0) {
            error!("Failed to send X11 session login request: {}", error);
            return Err(RouterError::TransportError("Failed to send X11 session login request".to_string()));
        }

        debug!("Waiting for X11 session login response");
        let mut response = zmq::Message::new();
        if let Err(error) = socket.recv(&mut response, 0) {
            error!("Failed to receive response to X11 session login request: {}", error);
            return Err(RouterError::TransportError("Failed to receive X11 session login request response".to_string()));
        }

        let response_message = response.as_str().unwrap();
        debug!("Received X11 session login request response: {}", &response_message);


        match serde_json::from_str::<SessionManagerResponse>(&response_message) {
            Ok(response) => match response {
                SessionManagerResponse::Login(session) => {
                    debug!("X11 session request successful, got display Id: {}", &session.display_id);
                    Ok(X11Session::new(session.id, session.username, session.display_id, session.xauthority_file_path))
                },
                SessionManagerResponse::Error { message } => {
                    debug!("X11 session login request failed, got error: {}", &message);
                    Err(RouterError::SessionError(format!("Failed to login to WebX Session Manager: {}", message)))
                },
                _ => {
                    debug!("X11 session login request return unknown response");
                    Err(RouterError::SessionError("Unkown response returned by WebX Session Manager".to_string()))
                }
            },
            Err(error) => {
                error!("Failed to unserialise WebX Session Manager login response: {}", error);
                Err(RouterError::SessionError("Failed to unserialise WebX Session Manager login response".to_string()))
            },
        }
    }

    fn handle_sesman_logout_request(&self, session_id: &str, socket: &zmq::Socket) -> Result<()> {
        // Create the request
        let request = SessionManagerRequest::Logout{id: session_id.to_string()};
        let request_message = serde_json::to_string(&request)?;

        // Send x11 session request
        debug!("Sending X11 session logout request");
        if let Err(error) = socket.send(&request_message, 0) {
            error!("Failed to send X11 session logout request: {}", error);
            return Err(RouterError::TransportError("Failed to send X11 session logout request".to_string()));
        }

        debug!("Waiting for X11 session logout response");
        let mut response = zmq::Message::new();
        if let Err(error) = socket.recv(&mut response, 0) {
            error!("Failed to receive response to X11 session lgout request: {}", error);
            return Err(RouterError::TransportError("Failed to receive X11 session logout request response".to_string()));
        }

        let response_message = response.as_str().unwrap();
        debug!("Received X11 session logout request response: {}", &response_message);

        match serde_json::from_str::<SessionManagerResponse>(&response_message) {
            Ok(response) => match response {
                SessionManagerResponse::Logout => {
                    debug!("X11 session logout request successful for session {}", session_id);
                    Ok(())
                },
                SessionManagerResponse::Error { message } => {
                    debug!("X11 session logout request failed for session {}, got error: {}", session_id, &message);
                    Err(RouterError::SessionError(format!("Failed to logout of WebX Session Manager: {}", message)))
                },
                _ => {
                    debug!("X11 session logout request return unknown response");
                    Err(RouterError::SessionError("Unkown response returned by WebX Session Manager".to_string()))
                }
            },
            Err(error) => {
                error!("Failed to unserialise WebX Session Manager logout response: {}", error);
                Err(RouterError::SessionError("Failed to unserialise WebX Session Manager login response".to_string()))
            },
        }
    }

    fn create_req_socket(&self, path: &str) -> Result<zmq::Socket> {
        let socket = self.context.socket(zmq::REQ)?;
        socket.set_linger(0)?;
        socket.set_rcvtimeo(15000)?;

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
