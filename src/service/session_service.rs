use crate::common::*;
use crate::service::{EngineValidator, SesmanConnector};

use uuid::Uuid;
use std::process::{Command, Child, Stdio};
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::fs::File;

use signal_child::Signalable;

pub struct Engine {
    process: Child,
    pub ipc: String, // specific req-rep address used to verify that the engine is running?
}

pub struct Session {
    pub id: Uuid,
    pub display_id: String,
    pub xauth_path: String,
    pub username: String,
    pub engine: Engine,
}

struct SessionManagerResponse {
    username: String,
    display_id: String,
    xauth_path: String,
}

pub struct SessionService {
    sessions: Vec<Session>,
    sesman_connector: SesmanConnector,
}

impl SessionService {

    pub fn new(context: zmq::Context) -> Self {
        Self {
            sessions: Vec::new(),
            sesman_connector: SesmanConnector::new(context),
        }
    }

    pub fn connect_to_sesman(&mut self, settings: &Settings) -> Result<()> {
        self.sesman_connector.open(&settings.transport.ipc.sesman_connector)
    }

    pub fn disconnect_from_sesman(&mut self) {
        self.sesman_connector.close();
    }

    pub fn create_session(&mut self, settings: &Settings, username: &str, password: &str, context: &zmq::Context) -> Result<&Session> {
        // See if we are using the session manager
        let ses_man_response;
        if settings.sesman.enabled {
            // Request display/session Id from WebX Session Manager
            ses_man_response = self.request_authenticated_x11_display(username, password)?;
            debug!("Got response for session manager: user \"{}\" has display on \"{}\"", ses_man_response.username, ses_man_response.display_id);
        
        } else {
            ses_man_response = self.get_fallback_x11_display(settings)?;
        }
        let display_id = ses_man_response.display_id;

        // See if session exists already: create if not
        if self.get_session(username, &display_id).is_none() {
            debug!("Creating session for user \"{}\" on display {}", username, display_id);
            // Create a session Id
            let session_id = Uuid::new_v4();

            // Spawn a new WebX Engine
            let engine = self.spawn_engine(&session_id, &display_id, &ses_man_response.xauth_path, settings)?;

            // Validate that the engine is running
            if let Err(error) = self.validate_engine(&engine, context) {
                error!("Failed to validate that WebX Engine is running for user {}: {}", username, error);
            }

            let session = Session {
                id: session_id,
                display_id: display_id.clone(),
                xauth_path: ses_man_response.xauth_path,
                username: username.to_string(),
                engine
            };

            // Store session
            self.sessions.push(session);

            debug!("Created session {} on display {} for user \"{}\"", session_id, display_id, username);

        } else {
            debug!("Session exists for user \"{}\" on display {}", username, display_id);
        }

        // Return the session
        return match self.get_session(username, &display_id) {
            Some(session) => Ok(session),
            None => Err(RouterError::SessionError(format!("Could not create retrieve Session for user \"{}\"", username)))
        };
    }

    pub fn stop_sessions(&mut self) {
        for session in self.sessions.iter_mut() {
            let engine = &mut session.engine;
            let process = &mut engine.process;
            match process.interrupt() {
                Ok(_) => {
                    debug!("Shutdown WebX Engine for {} on display {}", session.username, session.display_id);
                },
                Err(error) => error!("Failed to interrupt WebX Engine for {} running on PID {}: {}", session.username, process.id(), error),
            }
        }

        self.sessions.clear();
    }

    fn get_session(&self, username: &str, display_id: &str) -> Option<&Session> {
        self.sessions.iter().find(|&session| session.display_id == display_id && session.username == username)
    }

    fn get_fallback_x11_display(&self, settings: &Settings) -> Result<SessionManagerResponse> {
        let username = User::get_current_username()?;
        let display = &settings.sesman.fallback_display_id;
        Ok(SessionManagerResponse {
            username,
            display_id: display.to_string(),
            xauth_path: "".to_string(),
        })
    }

    fn request_authenticated_x11_display(&self, username: &str, password: &str) -> Result<SessionManagerResponse> {
        // Web service call to WebX Session Manager
        // TODO

        // Fake slow creation
        // thread::sleep(time::Duration::from_millis(2000));

        // let _response = self.sesman_connector.get_authenticated_x11_session(username, password);

        Ok(SessionManagerResponse {
            username: username.to_string(),
            display_id: ":0".to_string(),
            xauth_path: "".to_string(),
        })
    }

    fn spawn_engine(&self, session_uuid: &Uuid, display: &str, xauth_path: &str, settings: &Settings) -> Result<Engine> {
        let engine_path = &settings.engine.path;
        let engine_logdir = &settings.engine.logdir;
        let message_proxy_path = &settings.transport.ipc.message_proxy;
        let instruction_proxy_path = &settings.transport.ipc.instruction_proxy;
        let engine_connector_root_path = &settings.transport.ipc.engine_connector_root;

        let session_id = session_uuid.to_simple();

        // Get engine log path
        let log_path: String;
        if settings.sesman.enabled {
            log_path = format!("{}/webx-engine.{}.log", engine_logdir, session_id);
        
        } else {
            log_path = format!("{}/webx-engine.log", engine_logdir);
        }

        let file = File::create(log_path)?;
        let file_descriptor = file.into_raw_fd();
        let file_out = unsafe { Stdio::from_raw_fd(file_descriptor) };

        // Get engine connector IPC path
        let session_connector_path = format!("{}.{}.ipc", engine_connector_root_path, session_id);

        let mut command = Command::new(engine_path);
        command
            .stdout(file_out)
            .env("DISPLAY", display)
            .env("WEBX_ENGINE_LOG", "debug")
            .env("WEBX_ENGINE_IPC_SESSION_CONNECTOR_PATH", &session_connector_path)
            .env("WEBX_ENGINE_IPC_MESSAGE_PROXY_PATH", message_proxy_path)
            .env("WEBX_ENGINE_IPC_INSTRUCTION_PROXY_PATH", instruction_proxy_path)
            .env("WEBX_ENGINE_SESSION_ID", session_id.to_string());

        if settings.sesman.enabled {
            debug!("Launching WebX Engine \"{}\" on display {}", engine_path, display);
            command
                .env("XAUTHORITY", xauth_path);
        
        } else {
            debug!("Launching WebX Engine \"{}\" on display {}", engine_path, display);
        }

        match command.spawn() {
            Err(error) => Err(RouterError::SessionError(format!("Failed to spawn WebX Engine: {}", error))),
            Ok(child) => Ok(Engine {
                process: child,
                ipc: session_connector_path
            })
        }
    }

    fn validate_engine(&self, engine: &Engine, context: &zmq::Context) -> Result<()> {
        // Verify session is running
        let engine_validator = EngineValidator::new(context.clone());
        let mut retry = 3;
        let mut connection_error = "".to_string();
        while retry > 0 {
            match engine_validator.validate_connection(&engine.ipc) {
                Ok(_) => return Ok(()),
                Err(error) => {
                    connection_error = error.to_string();
                    retry -= 1;
                }
            }
        }
        Err(RouterError::SessionError(connection_error))
    }

}