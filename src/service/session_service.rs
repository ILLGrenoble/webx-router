use crate::common::*;

use uuid::Uuid;
use std::{thread, time};
use std::process::{Command, Child};
use std::os::unix::process::CommandExt;

pub struct Engine {
    process: Child,
    ipc: String, // specific req-rep address used to verify that the engine is running?
}

pub struct Session {
    pub id: Uuid,
    pub display_id: String,
    pub xauth_path: String,
    username: String,
    engine: Engine,
}

struct SessionManagerResponse {
    username: String,
    display_id: String,
    xauth_path: String,
}

pub struct SessionService {
    sessions: Vec<Session>,
}

impl SessionService {

    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
        }
    }

    pub fn create_session(&mut self, settings: &Settings, username: &str, password: &str) -> Result<&Session> {
        // Request display/session Id from WebX Session Manager
        let ses_man_response = self.request_authenticated_x11_display(username, password)?;
        let display_id = ses_man_response.display_id;

        // See if session exists already: create if not
        if self.get_session(&username, &display_id).is_none() {
            debug!("Creating session for user \"{}\" on display {}", username, display_id);
            // Create a session Id
            let session_id = Uuid::new_v4();

            // Spawn a new WebX Engine
            let engine = self.spawn_engine(&username, &session_id, &display_id, &ses_man_response.xauth_path, &settings)?;

            // Wait for engine process to start fully?
            // TODO

            let session = Session {
                id: session_id,
                display_id: display_id.clone(),
                xauth_path: ses_man_response.xauth_path,
                username: username.to_string(),
                engine: engine
            };

            // Store session
            self.sessions.push(session);
        
        } else {
            debug!("Session exists for user \"{}\" on display {}", username, display_id);
        }

        // Return the session
        match self.get_session(&username, &display_id) {
            Some(session) => return Ok(session),
            None => return Err(RouterError::SessionError(format!("Could not create retrieve Session for user \"{}\"", username)))
        };
    }

    pub fn stop_sesions(&self) {

    }

    fn get_session(&self, username: &str, display_id: &str) -> Option<&Session> {
        self.sessions.iter().find(|&session| session.display_id == display_id && session.username == username)
    }

    fn request_authenticated_x11_display(&self, username: &str, password: &str) -> Result<SessionManagerResponse> {
        // Web service call to WebX Session Manager
        // TODO

        // Fake slow creation
        thread::sleep(time::Duration::from_millis(2000));

        Ok(SessionManagerResponse {
            username: username.to_string(),
            display_id: ":0".to_string(),
            xauth_path: "".to_string(),
        })
    }

    fn spawn_engine(&self, username: &str, session_uuid: &Uuid, display: &str, xauth_path: &str, settings: &Settings) -> Result<Engine> {
        let engine_path = &settings.engine.path;
        let message_proxy_addr = &settings.transport.ipc.message_proxy;
        let instruction_proxy_addr = &settings.transport.ipc.instruction_proxy;

        // TODO redirect output to file?

        // Get UID of user to run process as
        let uid = User::get_uid_for_username(username)?;
        debug!("Launching WebX Engine \"{}\" as user \"{}\" ({}) on display {}", engine_path, username, uid, display);

        match Command::new(engine_path)
            .uid(uid)
            .env("DISPLAY", display)
            .env("XAUTHORITY", xauth_path)
            .env("WEBX_ENGINE_IPC_MESSAGE_PROXY_ADDRESS", message_proxy_addr)
            .env("WEBX_ENGINE_IPC_INSTRUCTION_PROXY_ADDRESS", instruction_proxy_addr)
            .env("WEBX_ENGINE_SESSION_ID", session_uuid.to_simple().to_string())
            .spawn() {
                Err(error) => Err(RouterError::SessionError(format!("Failed to spawn WebX Engine: {}", error))),
                Ok(child) => Ok(Engine {
                    process: child,
                    ipc: "".to_string()
                })
            }
    }

}