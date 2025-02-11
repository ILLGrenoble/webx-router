use crate::common::*;
use crate::service::{EngineValidator, SesmanConnector};

use uuid::Uuid;
use std::process::{Command, Stdio};
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::fs::File;

pub struct SessionService {
    session_container: SessionContainer,
}

impl SessionService {

    pub fn new() -> Self {
        Self {
            session_container: SessionContainer::new(),
        }
    }

    pub fn stop_sessions(&mut self) {
        self.session_container.stop_sessions();
    }

    pub fn get_or_create_session(&mut self, settings: &Settings, username: &str, password: &str, width: u32, height: u32, keyboard: &str, context: &zmq::Context) -> Result<&Session> {
        // See if we are using the session manager
        let x11_session;
        if settings.sesman.enabled {
            // Request display/session Id from WebX Session Manager
            x11_session = self.request_authenticated_x11_display(username, password, width, height, context, settings)?;
            debug!("Got response for session manager: user \"{}\" has display on \"{}\"", x11_session.username(), x11_session.display_id());
        
        } else {
            x11_session = self.get_fallback_x11_display(settings)?;
        }

        // See if session already exists matching x11_session attributes
        if self.session_container.get_session_by_x11session(&x11_session).is_none() {
            // cleanup any other sessions for the user
            self.session_container.remove_session_for_user(username);

            // Create new session for the user
            self.create_session(x11_session, settings, keyboard, context)?;
        } 

        // Return the session
        return match self.session_container.get_session_by_username(username) {
            Some(session) => Ok(session),
            None => Err(RouterError::SessionError(format!("Could not retrieve Session for user \"{}\"", username)))
        };
    }

    pub fn ping_session(&mut self, session_id: &str, context: &zmq::Context) -> Result<()> {
        if let Some(session) = self.session_container.get_session_by_session_id(session_id) {
            if let Err(error) =  self.validate_engine(session.engine(), context, 1) {
                // Delete session
                self.session_container.remove_session_with_id(session_id);
                return Err(error);
            }

        } else {
            return Err(RouterError::SessionError(format!("Could not retrieve Session with ID \"{}\"", session_id)));
        }

        // All good
        Ok(())
    }

    pub fn update_session_activity(&mut self, session_id: &str) {
        if let Some(session) = self.session_container.get_mut_session_by_session_id(session_id) {
            session.update_activity();
        }
    }

    pub fn cleanup_inactive_sessions(&mut self, settings: &Settings, context: &zmq::Context) {
        if settings.sesman.auto_logout_s > 0 {
            let inactive_sessions = self.session_container.get_inactive_session_ids(settings.sesman.auto_logout_s);
            for session in inactive_sessions.iter() {
                info!("Removing inactive session with id {} for user {}", &session.0, &session.1);
    
                // Remove session
                self.session_container.remove_session_with_id(&session.0);
    
                // Close X11 session
                if settings.sesman.enabled {
                    self.request_session_logout(&session.0, context, settings);
                }
            }
        }
    }

    fn create_session(&mut self, x11_session: X11Session, settings: &Settings, keyboard: &str, context: &zmq::Context)  -> Result<()> {
        debug!("Creating session for user \"{}\" on display {}", &x11_session.username(), &x11_session.display_id());

        // Spawn a new WebX Engine
        let engine = self.spawn_engine(&x11_session, settings, keyboard)?;

        let mut session = Session::new(x11_session, engine);

        // Validate that the engine is running
        if let Err(error) = self.validate_engine(session.engine(), context, 3) {
            // Make sure the engine process has stopped
            session.stop();
            return Err(RouterError::SessionError(format!("Failed to validate that WebX Engine is running for user {}: {}", session.username(), error)));
        }

        debug!("Created session {} on display {} for user \"{}\"", &session.id(), &session.display_id(), &session.username());

        // Store session
        self.session_container.add_session(session);

        Ok(())
    }

    fn get_fallback_x11_display(&self, settings: &Settings) -> Result<X11Session> {
        let session_id = Uuid::new_v4().simple().to_string();
        let username = System::get_current_username()?;
        let display = &settings.sesman.fallback_display_id;
        Ok(X11Session::new(session_id, username, display.to_string(), "".to_string()))
    }

    fn request_authenticated_x11_display(&self, username: &str, password: &str, width: u32, height: u32, context: &zmq::Context, settings: &Settings) -> Result<X11Session> {
        // Call to WebX Session Manager
        let sesman_connector = SesmanConnector::new(context.clone());

        sesman_connector.get_authenticated_x11_session(username, password, width, height, &settings.transport.ipc.sesman_connector)
    }

    fn request_session_logout(&self, session_id: &str, context: &zmq::Context, settings: &Settings) {
        // Call to WebX Session Manager
        let sesman_connector = SesmanConnector::new(context.clone());

        if let Err(error) = sesman_connector.logout(session_id, &settings.transport.ipc.sesman_connector) {
            warn!("Got error logging out X11 session: {}", error);
        }
    }

    fn spawn_engine(&self, x11_session: &X11Session, settings: &Settings, keyboard: &str) -> Result<Engine> {
        let engine_path = &settings.engine.path;
        let engine_logdir = &settings.engine.logdir;
        let message_proxy_path = &settings.transport.ipc.message_proxy;
        let instruction_proxy_path = &settings.transport.ipc.instruction_proxy;
        let engine_connector_root_path = &settings.transport.ipc.engine_connector_root;

        // Get engine log path
        let log_path: String;
        if settings.sesman.enabled {
            log_path = format!("{}/webx-engine.{}.log", engine_logdir, x11_session.session_id());
        
        } else {
            log_path = format!("{}/webx-engine.log", engine_logdir);
        }

        let file = File::create(log_path)?;
        let file_descriptor = file.into_raw_fd();
        let file_out = unsafe { Stdio::from_raw_fd(file_descriptor) };

        // Get engine connector IPC path
        let session_connector_path = format!("{}.{}.ipc", engine_connector_root_path, x11_session.session_id());

        let mut command = Command::new(engine_path);
        command
            .arg("-k")
            .arg(keyboard)
            .stdout(file_out)
            .env("DISPLAY", x11_session.display_id())
            .env("WEBX_ENGINE_LOG", "debug")
            .env("WEBX_ENGINE_IPC_SESSION_CONNECTOR_PATH", &session_connector_path)
            .env("WEBX_ENGINE_IPC_MESSAGE_PROXY_PATH", message_proxy_path)
            .env("WEBX_ENGINE_IPC_INSTRUCTION_PROXY_PATH", instruction_proxy_path)
            .env("WEBX_ENGINE_SESSION_ID", x11_session.session_id());

        if settings.sesman.enabled {
            debug!("Launching WebX Engine \"{}\" on display {}", engine_path, x11_session.display_id());
            command
                .env("XAUTHORITY", x11_session.xauthority_file_path());
        
        } else {
            debug!("Launching WebX Engine \"{}\" on display {}", engine_path, x11_session.display_id());
        }

        debug!("Spawning command: {}", format!("{:?}", command).replace("\"", ""));

        match command.spawn() {
            Err(error) => Err(RouterError::SessionError(format!("Failed to spawn WebX Engine: {}", error))),
            Ok(child) => Ok(Engine::new(child, session_connector_path))
        }
    }

    fn validate_engine(&self, engine: &Engine, context: &zmq::Context, mut tries: i32) -> Result<()> {
        // Verify session is running
        let engine_validator = EngineValidator::new(context.clone());
        let mut connection_error = "".to_string();
        while tries > 0 {
            match engine_validator.validate_connection(&engine.ipc()) {
                Ok(_) => return Ok(()),
                Err(error) => {
                    connection_error = error.to_string();
                    tries -= 1;
                }
            }
        }
        Err(RouterError::SessionError(connection_error))
    }

}