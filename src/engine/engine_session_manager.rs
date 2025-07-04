use crate::{
    authentication::{AuthenticatedSession},
    common::{RouterError, Result, Settings},
    sesman::{X11Session, X11SessionManager}
};
use super::{EngineService, EngineSession, Engine, SessionConfig};
use std::{
    thread,
    time,
    sync::Mutex,
};
use uuid::Uuid;

/// The `EngineSessionManager` manages user WebX sessions, including creating, stopping,
/// and validating sessions. It interacts with the WebX Session Manager and the WebX Engine.
pub struct EngineSessionManager {
    settings: Settings,
    x11_session_manager: X11SessionManager,
    engine_service: EngineService,
    sessions: Mutex<Vec<EngineSession>>,
}

impl EngineSessionManager {
    /// Creates a new `EngineSessionManager` instance.
    ///
    /// # Arguments
    /// * `settings` - The settings.
    ///
    /// # Returns
    /// * `EngineSessionManager` - A new instance.
    pub fn new(settings: &Settings) -> Self {
        Self {
            settings: settings.clone(),
            x11_session_manager: X11SessionManager::new(&settings.sesman),
            engine_service: EngineService::new(),
            sessions: Mutex::new(Vec::new()),
        }
    }

    /// Stops all active engines and clears all sessions.
    pub fn shutdown(&mut self) {
        if let Ok(mut sessions) = self.sessions.lock() {
            for session in sessions.iter_mut() {
               session.stop_engine();
            }
            sessions.clear();
        } else {
           error!("Failed to obtain sessions mutex to kill all Engine Sessions during shutdown");
        }
        
        if let Err(error) = self.x11_session_manager.kill_all() {
           error!("Failed to kill all X11 sessions during shutdown: {}", error);
        }
    }

    /// Retrieves all X11 sessions.
    ///
    /// # Returns
    /// * `Option<Vec<X11Session>>` - Some vector of sessions if available, or None.
    pub fn get_all_x11_sessions(&self) -> Option<Vec<X11Session>> {
        self.x11_session_manager.get_all()
    }

    /// Retrieves or creates a session for a user based on the provided settings and credentials.
    /// A new WebX Engine process is spawned if necessary.
    ///
    /// # Arguments
    /// * `authenticated_session` - The authenticated user session (account and environment).
    /// * `session_config` - The session config (screen resolution, keyboard layout, additional parameters).
    /// * `context` - The ZeroMQ context.
    ///
    /// # Returns
    /// A reference to the created or retrieved session.
    pub fn get_or_create_engine_session(&mut self, authenticated_session: AuthenticatedSession, session_config: SessionConfig, context: &zmq::Context) -> Result<String> {
        // Request display/session Id from WebX Session Manager
        let x11_session = self.x11_session_manager.create_session(&authenticated_session, session_config.resolution().clone())?;

        debug!("X11 session obtained for user \"{}\" on display \"{}\"", x11_session.account().username(), x11_session.display_id());

        if let Ok(mut sessions) = self.sessions.lock() {
            // See if session already exists matching x11_session attributes
            if let Some(session) = sessions.iter().find(|session| 
                session.username() == x11_session.account().username() && 
                session.id() == x11_session.id() && 
                session.display_id() == x11_session.display_id()) {

                info!("Found existing Engine Session for user \"{}\" on display \"{}\" with id \"{}\"", session.username(), session.display_id(), session.id());
                return Ok(session.secret().to_string());
            }

            // Remove existing sessions for the user
            if let Some((index, session)) = sessions.iter_mut().enumerate().find(|(_, session)| session.username() == x11_session.account().username()) {
                debug!("Removing existing Engine Session for user \"{}\" on display \"{}\" with id \"{}\"", session.username(), session.display_id(), session.id());
                // stop the engine session
                session.stop_engine();

                // Remove the old engine session
                sessions.remove(index);        
            }
        } else {
            return Err(RouterError::EngineSessionError(format!("Failed to get session lock")));
        }

        // Create new session for the user
        self.create_engine_session(x11_session, &session_config, context)?;

        if let Ok(sessions) = self.sessions.lock() {
            // Return the newly created session
            match sessions.iter().find(|session| session.username() == authenticated_session.account().username()) {
                Some(session) => Ok(session.secret().to_string()),
                None => Err(RouterError::EngineSessionError(format!("Could not retrieve Engine Session for user \"{}\"", authenticated_session.account().username())))
            }
        } else {
            Err(RouterError::EngineSessionError(format!("failed to get session lock")))
        }
    }

    /// Pings a WebX Engine to check if it is active.
    ///
    /// # Arguments
    /// * `secret` - The secret of the session to ping.
    ///
    /// # Returns
    /// * `Result<()>` - Ok if the engine is active, Err otherwise.
    pub fn ping_engine(&mut self, secret: &str) -> Result<()> {
        if let Ok(mut sessions) = self.sessions.lock() {
            let (index, session) = sessions.iter_mut().enumerate().find(|(_, session)| session.secret() == secret)
                .ok_or_else(|| RouterError::EngineSessionError(format!("Could not retrieve Engine Session by provided secret")))?;

            match self.engine_service.validate_engine(session.engine_mut(), 1) {
                Ok(_) => Ok(()),
                Err(error) => {
                    // stop the engine session (if possible)
                    session.stop_engine();

                    // Remove the old engine session
                    sessions.remove(index);   

                    Err(error)
                }
            }
        } else {
            Err(RouterError::EngineSessionError(format!("Failed to get sessions lock")))
        }
    }

    /// Sends a request to a WebX Engine and retrieves the response.
    ///
    /// # Arguments
    /// * `secret` - The secret of the session.
    /// * `request` - The request string to send.
    ///
    /// # Returns
    /// * `Result<String>` - The response from the session, or an error.
    pub fn send_engine_request(&mut self, secret: &str, request: &str) -> Result<String> {
        if let Ok(mut sessions) = self.sessions.lock() {
            let session = sessions.iter_mut().find(|session| session.secret() == secret)
                .ok_or_else(|| RouterError::EngineSessionError(format!("Could not retrieve Engine Session with provided secret")))?;

            self.engine_service.send_engine_request(session.engine_mut(), request)

        } else {
            Err(RouterError::EngineSessionError(format!("Failed to get sessions lock")))
        }
    }

    /// Creates a new session for a user. This spawns a new WebX Engine process if necessary.
    ///
    /// # Arguments
    /// * `x11_session` - The X11 session details.
    /// * `session_config` - The session config (keyboard layout, additional parameters).
    /// * `context` - The ZeroMQ context.
    ///
    /// # Returns
    /// A result indicating success or failure.
    fn create_engine_session(&mut self, x11_session: X11Session, session_config: &SessionConfig, context: &zmq::Context) -> Result<()> {
        info!("Creating Engine Session for user \"{}\" on display \"{}\" with id \"{}\"", &x11_session.account().username(), &x11_session.display_id(), x11_session.id());

        let secret = Uuid::new_v4().simple().to_string();

        // Spawn a new WebX Engine
        if let Some(engine) = self.multi_try_spawn_engine(&x11_session, &secret, context, session_config, 3) {

            let mut session = EngineSession::new(secret, x11_session, engine);

            // Validate that the engine is running
            if let Err(error) = self.engine_service.validate_engine(session.engine_mut(), 3) {
                // Make sure the engine process has stopped
                session.stop_engine();
                return Err(RouterError::EngineSessionError(format!("Failed to validate that WebX Engine is running for user \"{}\" with session id \"{}\": {}", session.username(), session.id(), error)));
            }

            debug!("Created session with id \"{}\" on display \"{}\" for user \"{}\"", session.id(), session.display_id(), session.username());

            // Store session
            if let Ok(mut sessions) = self.sessions.lock() {
                sessions.push(session);
            }

            Ok(())
        } else {
            Err(RouterError::EngineSessionError(format!("Failed to launch WebX Engine for user \"{}\" with session id \"{}\"", x11_session.account().username(), x11_session.id())))
        }
    }

    /// Attempts to spawn a WebX Engine process multiple times until successful or the maximum number of tries is reached.
    ///
    /// # Arguments
    /// * `x11_session` - The X11 session details.
    /// * `context` - The ZeroMQ context.
    /// * `session_config` - The session config (keyboard layout, additional parameters).
    /// * `tries` - The maximum number of attempts.
    ///
    /// # Returns
    /// * `Option<Engine>` - Some(Engine) if successful, None otherwise.
    fn multi_try_spawn_engine(&self, x11_session: &X11Session, secret: &str, context: &zmq::Context, session_config: &SessionConfig, tries: u64) -> Option<Engine> {
        let mut attempt = 0;
        while attempt < tries {
            debug!("Starting WebX Engine for user \"{}\" with session id \"{}\" on display \"{}\" (attempt {} / {})", x11_session.account().username(), x11_session.id(), x11_session.display_id(), attempt + 1, tries);
            match self.engine_service.spawn_engine(x11_session, secret, context, &self.settings, session_config) {
                Ok(engine) => {
                    thread::sleep(time::Duration::from_millis(attempt * 2000));

                    if engine.is_running().unwrap_or(true) {
                        debug!("WebX Engine running for user \"{}\" with session id \"{}\" on display \"{}\"", x11_session.account().username(), x11_session.id(), x11_session.display_id());
                        return Some(engine);
                    }

                    warn!("WebX Engine terminated prematurely for user \"{}\" with session id \"{}\"", x11_session.account().username(), x11_session.id());

                },
                Err(error) => {
                    error!("Failed to spawn WebX Engine for user \"{}\" with session id \"{}\": {}", x11_session.account().username(), x11_session.id(), error);
                    return None;
                }
            }
            attempt += 1;
        }
        None
    }
}