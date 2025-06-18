use crate::{
    authentication::{Authenticator, Credentials},
    common::{RouterError, Result, EngineSessionContainer, SesManSettings, Settings, EngineSession, Engine, to_snake_case, System},
    service::{EngineConnector},
    sesman::{X11Session, ScreenResolution, X11SessionManager, XorgService}
};

use std::process::{Command, Stdio};
use std::os::unix::{
    io::{FromRawFd, IntoRawFd},
    prelude::CommandExt,
};
use std::collections::HashMap;
use std::fs::OpenOptions;

/// The `EngineSessionService` manages user WebX sessions, including creating, stopping,
/// and validating sessions. It interacts with the WebX Session Manager and the WebX Engine.
pub struct EngineSessionService {
    session_container: EngineSessionContainer,
    x11_session_manager: X11SessionManager,
}

impl EngineSessionService {
    /// Creates a new `SessionService` instance.
    pub fn new(settings: &SesManSettings) -> Self {
        let session_container = EngineSessionContainer::new();
        let authenticator = Authenticator::new(settings.authentication.service.to_owned());
        let xorg_service = XorgService::new(settings.xorg.to_owned());
        let x11_session_manager = X11SessionManager::new(authenticator, xorg_service);
        Self {
            session_container,
            x11_session_manager,
        }
    }

    /// Stops all active engines and .
    pub fn shutdown(&mut self) {
        self.session_container.stop_engines();
        if let Err(error) = self.x11_session_manager.kill_all() {
           error!("Failed to kill all X11 sessions during shutdown: {}", error);
        }
    }

    pub fn get_all_x11_sessions(&self) -> Option<Vec<X11Session>> {
        self.x11_session_manager.get_all()
    }

    /// Retrieves or creates a session for a user based on the provided settings and credentials.
    /// A new WebX Engine process is spawned if necessary.
    ///
    /// # Arguments
    /// * `settings` - The application settings.
    /// * `credentials` - The credentials of the user.
    /// * `resolution` - The screen resolution of the session display.
    /// * `keyboard` - The keyboard layout.
    /// * `context` - The ZeroMQ context.
    ///
    /// # Returns
    /// A reference to the created or retrieved session.
    pub fn get_or_create_engine_session(&mut self, settings: &Settings, credentials: &Credentials, resolution: ScreenResolution, keyboard: &str, engine_parameters: &HashMap<String, String>, context: &zmq::Context) -> Result<&EngineSession> {
        // Request display/session Id from WebX Session Manager
        match self.x11_session_manager.create_session(credentials, resolution) {
            Ok(x11_session) => {
                debug!("Got response for session manager: user \"{}\" has display on \"{}\"", x11_session.account().username(), x11_session.display_id());

                // See if session already exists matching x11_session attributes
                if self.session_container.get_engine_session_by_x11_session(&x11_session).is_none() {
                    // cleanup any other sessions for the user
                    self.session_container.remove_engine_session_for_user(credentials.username());

                    // Create new session for the user
                    self.create_engine_session(x11_session, settings, keyboard, engine_parameters, context)?;
                } 

                // Return the session
                return match self.session_container.get_engine_session_by_username(credentials.username()) {
                    Some(session) => Ok(session),
                    None => Err(RouterError::EngineSessionError(format!("Could not retrieve Session for user \"{}\"", credentials.username())))
                };
            },
            Err(error) => {
                return Err(RouterError::EngineSessionError(format!("Failed to create X11 session for user {}: {}", credentials.username(), error)));
            }
        };
    }

    /// Pings a WebX Engine to check if it is active.
    ///
    /// # Arguments
    /// * `session_id` - The ID of the session to ping.
    /// * `context` - The ZeroMQ context.
    ///
    /// # Returns
    /// A result indicating success or failure.
    pub fn ping_engine(&mut self, session_id: &str, context: &zmq::Context) -> Result<()> {
        if let Some(session) = self.session_container.get_engine_session_by_session_id(session_id) {
            if let Err(error) =  self.validate_engine(session.engine(), context, 1) {
                // Delete session
                self.session_container.remove_engine_session_with_id(session_id);
                return Err(error);
            }

        } else {
            return Err(RouterError::EngineSessionError(format!("Could not retrieve Session with ID \"{}\"", session_id)));
        }

        // All good
        Ok(())
    }

    /// Sends a request to a WebX Engine and retrieves the response.
    ///
    /// # Arguments
    /// * `session_id` - The ID of the session.
    /// * `context` - The ZeroMQ context.
    /// * `request` - The request string to send.
    ///
    /// # Returns
    /// The response from the session.
    pub fn send_engine_request(&mut self, session_id: &str, context: &zmq::Context, request: &str) -> Result<String> {
        if let Some(session) = self.session_container.get_engine_session_by_session_id(session_id) {

            let engine_connector = EngineConnector::new(context.clone());
            return engine_connector.send_request(&session.engine().ipc(), request);
        }

        Err(RouterError::EngineSessionError(format!("Could not retrieve Session with ID \"{}\"", session_id)))
    }

    /// Updates the activity timestamp of a session.
    ///
    /// # Arguments
    /// * `session_id` - The ID of the session to update.
    pub fn update_engine_session_activity(&mut self, session_id: &str) {
        if let Some(session) = self.session_container.get_mut_engine_session_by_session_id(session_id) {
            session.update_activity();
        }
    }

    /// Cleans up inactive sessions based on the inactivity timeout in the settings.
    ///
    /// # Arguments
    /// * `settings` - The application settings.
    /// * `context` - The ZeroMQ context.
    pub fn cleanup_inactive_engine_sessions(&mut self, settings: &Settings) {
        if settings.sesman.auto_logout_s > 0 {
            let inactive_sessions = self.session_container.get_inactive_session_ids(settings.sesman.auto_logout_s);
            for session in inactive_sessions.iter() {
                info!("Removing inactive session with id {} for user {}", &session.0, &session.1);
    
                // Remove session
                self.session_container.remove_engine_session_with_id(&session.0);
    
                // Close X11 session
                if let Err(error) = self.x11_session_manager.kill_by_id(&session.0) {
                   error!("Could not logout x11 session: {}", error);
                }
            }
        }
    }

    /// Creates a new session for a user. This spawns a new WebX Engine process if necessary.
    ///
    /// # Arguments
    /// * `x11_session` - The X11 session details.
    /// * `settings` - The application settings.
    /// * `keyboard` - The keyboard layout.
    /// * `context` - The ZeroMQ context.
    ///
    /// # Returns
    /// A result indicating success or failure.
    fn create_engine_session(&mut self, x11_session: X11Session, settings: &Settings, keyboard: &str, engine_parameters: &HashMap<String, String>, context: &zmq::Context)  -> Result<()> {
        debug!("Creating session for user \"{}\" on display {}", &x11_session.account().username(), &x11_session.display_id());

        // Spawn a new WebX Engine
        let engine = self.spawn_engine(&x11_session, settings, keyboard, engine_parameters)?;

        let mut session = EngineSession::new(x11_session, engine);

        // Validate that the engine is running
        if let Err(error) = self.validate_engine(session.engine(), context, 3) {
            // Make sure the engine process has stopped
            session.stop_engine();
            return Err(RouterError::EngineSessionError(format!("Failed to validate that WebX Engine is running for user {}: {}", session.username(), error)));
        }

        debug!("Created session {} on display {} for user \"{}\"", &session.id(), &session.display_id(), &session.username());

        // Store session
        self.session_container.add_engine_session(session);

        Ok(())
    }



    /// Spawns a new WebX Engine process for a session.
    ///
    /// # Arguments
    /// * `x11_session` - The X11 session details.
    /// * `settings` - The application settings.
    /// * `keyboard` - The keyboard layout.
    ///
    /// # Returns
    /// The spawned WebX Engine instance.
    fn spawn_engine(&self, x11_session: &X11Session, settings: &Settings, keyboard: &str, engine_parameters: &HashMap<String, String>) -> Result<Engine> {
        let engine_path = &settings.engine.path;
        let engine_log_path = &settings.engine.log_path;
        let message_proxy_path = &settings.transport.ipc.message_proxy;
        let instruction_proxy_path = &settings.transport.ipc.instruction_proxy;
        let engine_connector_root_path = &settings.transport.ipc.engine_connector_root;

        // Get engine log path
        let log_path = format!("{}/webx-engine.{}.log", engine_log_path, x11_session.id());

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;

        let file_descriptor = file.into_raw_fd();
        let file_out = unsafe { Stdio::from_raw_fd(file_descriptor) };

        // Get engine connector IPC path
        let session_connector_path = format!("{}.{}.ipc", engine_connector_root_path, x11_session.id());

        let webx_user = System::get_user("webx")
            .ok_or_else(|| RouterError::EngineSessionError("Failed to retrieve 'webx' user".to_string()))?;

        let mut command = Command::new(engine_path);
        command
            .arg("-k")
            .arg(keyboard)
            .stdout(file_out)
            .env("DISPLAY", x11_session.display_id())
            .env("XAUTHORITY", x11_session.xauthority_file_path())
            .env("WEBX_ENGINE_LOG_LEVEL", "debug")
            .envs(self.convert_engine_parameters(engine_parameters))
            .env("WEBX_ENGINE_IPC_SESSION_CONNECTOR_PATH", &session_connector_path)
            .env("WEBX_ENGINE_IPC_MESSAGE_PROXY_PATH", message_proxy_path)
            .env("WEBX_ENGINE_IPC_INSTRUCTION_PROXY_PATH", instruction_proxy_path)
            .env("WEBX_ENGINE_SESSION_ID", x11_session.id())
            .uid(webx_user.uid.as_raw())
            .gid(webx_user.gid.as_raw());

        debug!("Launching WebX Engine \"{}\" on display {}", engine_path, x11_session.display_id());

        debug!("Spawning command: {}", format!("{:?}", command).replace("\"", ""));

        match command.spawn() {
            Err(error) => Err(RouterError::EngineSessionError(format!("Failed to spawn WebX Engine: {}", error))),
            Ok(child) => Ok(Engine::new(child, session_connector_path))
        }
    }

    /// Validates that a WebX Engine is running and responsive.
    ///
    /// # Arguments
    /// * `engine` - The WebX Engine instance.
    /// * `context` - The ZeroMQ context.
    /// * `tries` - The number of validation attempts.
    ///
    /// # Returns
    /// A result indicating success or failure.
    fn validate_engine(&self, engine: &Engine, context: &zmq::Context, mut tries: i32) -> Result<()> {
        // Verify session is running
        let engine_connector = EngineConnector::new(context.clone());
        let mut connection_error = "".to_string();
        while tries > 0 {
            match engine_connector.send_request(&engine.ipc(), "ping") {
                Ok(message) => {
                    if message != "pong" {
                        error!("Received non-pong response from WebX Engine on {}: {}", &engine.ipc(), message);
                        return Err(RouterError::EngineSessionError("Received non-pong message".to_string()));
                    }
            
                    trace!("Received pong response from WebX Engine on {}", &engine.ipc());
                    return Ok(());
                },
                Err(error) => {
                    connection_error = error.to_string();
                    tries -= 1;
                }
            }
        }
        Err(RouterError::EngineSessionError(connection_error))
    }

    /// Converts engine parameters into environment variables.
    /// Keys are converted from camelCase to SNAKE_CASE and prefixed with "WEBX_ENGINE_".
    ///
    /// # Arguments
    /// * `parameters` - HashMap containing the engine parameters
    ///
    /// # Returns
    /// A vector of tuples containing the environment variable name and value
    fn convert_engine_parameters(&self, parameters: &HashMap<String, String>) -> Vec<(String, String)> {
        parameters
            .iter()
            .map(|(key, value)| {
                let snake_case = to_snake_case(key);
                let env_key = format!("WEBX_ENGINE_{}", snake_case.to_uppercase());
                (env_key, value.clone())
            })
            .collect()
    }

}