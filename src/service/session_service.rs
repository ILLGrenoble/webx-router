use crate::common::*;

use uuid::Uuid;
use std::{thread, time};
use std::process::{Command, Child};

pub struct Engine {
    process: Child,
    ipc: String, // specific req-rep address used to verify that the engine is running?
}

pub struct Session {
    pub id: Uuid,
    pub display: String,
    username: String,
    engine: Engine,
}

struct SessionManagerResponse {
    session_id: String,
    display_id: String
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
        let session_uuid = match Uuid::parse_str(ses_man_response.session_id.as_str()) {
            Err(error) => return Err(RouterError::SessionError(format!("Could not parse UUID from Session Manager: {}", error))),
            Ok(uuid) => uuid
        };

        // See if session exists already
        if self.get_session(&session_uuid).is_none() {
            // Spawn a new WebX Engine
            let engine = self.spawn_engine(&session_uuid, &ses_man_response.display_id, &settings)?;

            // Wait for engine process to start fully?
            // TODO

            let session = Session {
                id: session_uuid,
                display: ses_man_response.display_id,
                username: username.to_string(),
                engine: engine
            };

            // Store session
            self.sessions.push(session);
        }

        // session alreayd exists so return it
        match self.get_session(&session_uuid) {
            Some(session) => return Ok(session),
            None => return Err(RouterError::SessionError("Could not create retrieve Session".to_string()))
        };
    }

    fn get_session(&self, session_uuid: &Uuid) -> Option<&Session> {
        self.sessions.iter().find(|&session| session.id == *session_uuid)
    }

    fn request_authenticated_x11_display(&self, username: &str, password: &str) -> Result<SessionManagerResponse> {
        // Web service call to WebX Session Manager
        // TODO

        let session_id = Uuid::new_v4();

        // Fake slow creation
        thread::sleep(time::Duration::from_millis(2000));

        Ok(SessionManagerResponse {
            session_id: session_id.to_string(),
            display_id: ":0".to_string()
        })
    }

    fn spawn_engine(&self, session_uuid: &Uuid, display: &str, settings: &Settings) -> Result<Engine> {
        let engine_path = &settings.engine.path;
        let message_proxy_addr = &settings.transport.ipc.message_proxy;
        let instruction_proxy_addr = &settings.transport.ipc.instruction_proxy;

        // TODO redirect output to file?

        match Command::new(engine_path)
            .env("DISPLAY", display)
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