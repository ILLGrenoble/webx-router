use crate::{
    authentication::{AuthenticatedSession},
    common::{RouterError, Result, Settings},
    sesman::{X11Session, X11SessionManager}
};
use super::{EngineService, EngineSession, Engine, SessionConfig, SessionCreationProcess, EngineSessionInfo, EngineStatus};
use std::{
    thread,
    time,
};
use uuid::Uuid;
use time::Duration;

/// The `EngineSessionManager` manages user WebX sessions, including creating, stopping,
/// and validating sessions. It interacts with the WebX Session Manager and the WebX Engine.
pub struct EngineSessionManager {
    settings: Settings,
    context: zmq::Context,
    x11_session_manager: X11SessionManager,
    engine_service: EngineService,
    sessions: Vec<EngineSession>,
    creation_processes: Vec<SessionCreationProcess>,
}

impl EngineSessionManager {
    /// Creates a new `EngineSessionManager` instance.
    ///
    /// # Arguments
    /// * `settings` - The settings.
    /// * `context` - The ZeroMQ context.
    ///
    /// # Returns
    /// * `EngineSessionManager` - A new instance.
    pub fn new(settings: &Settings, context: zmq::Context) -> Self {
        Self {
            settings: settings.clone(),
            context: context,
            x11_session_manager: X11SessionManager::new(&settings.sesman),
            engine_service: EngineService::new(),
            sessions: Vec::new(),
            creation_processes: Vec::new(),
        }
    }

    /// Stops all active engines and clears all sessions.
    pub fn shutdown(&mut self) {
        for session in self.sessions.iter_mut() {
            session.stop_engine();
        }
        self.sessions.clear();
        
        if let Err(error) = self.x11_session_manager.kill_all() {
           error!("Failed to kill all X11 sessions during shutdown: {}", error);
        }
    }

    /// Retrieves all X11 sessions.
    ///
    /// # Returns
    /// * `Option<Vec<X11Session>>` - vector of sessions.
    pub fn get_all_x11_sessions(&self) -> Vec<X11Session> {
        self.x11_session_manager.sessions()
    }

    pub fn get_or_create_x11_and_engine_session_async(&mut self, authenticated_session: AuthenticatedSession, session_config: SessionConfig) -> Result<EngineSessionInfo> {
        // Request display/session Id from WebX Session Manager
        let x11_session = self.x11_session_manager.get_or_create_x11_session_async(&authenticated_session, session_config.resolution().clone())?;
        
        if x11_session.window_manager().is_some() {
            // X11 session is complete: we just need to check if the engine session already exists
            return match self.get_or_create_engine_session(&x11_session, session_config) {
                Ok(secret) => {
                    debug!("Engine session obtained for user \"{}\" on display \"{}\" with id \"{}\"", x11_session.account().username(), x11_session.display_id(), x11_session.id());
                    Ok(EngineSessionInfo::new(secret, EngineStatus::Ready))
                },
                Err(error) => Err(error),
            }
        }

        if let Some(process) = self.creation_processes.iter().find(|process| process.session_id() == x11_session.id()) {
            // If a creation process is already running for this session, return the existing process
            debug!("Creation process already running for session id \"{}\" on display \"{}\"", x11_session.id(), x11_session.display_id());
            return Ok(EngineSessionInfo::new(process.secret().to_string(), EngineStatus::Starting));

        } else {
            info!("Starting new creation process for session id \"{}\" on display \"{}\"", x11_session.id(), x11_session.display_id());
            let creation_process = SessionCreationProcess::new(
                x11_session.id().to_string(),
                x11_session.account().username().to_string(),
                x11_session.display_id().to_string(),
                session_config,
                Uuid::new_v4().simple().to_string(),
            );
            self.creation_processes.push(creation_process);
            Ok(EngineSessionInfo::new(
                self.creation_processes.last().unwrap().secret().to_string(),
                EngineStatus::Starting,
            ))            
        }
    }

    /// Retrieves or creates a session for a user based on the provided settings and credentials.
    /// A new WebX Engine process is spawned if necessary.
    ///
    /// # Arguments
    /// * `authenticated_session` - The authenticated user session (account and environment).
    /// * `session_config` - The session config (screen resolution, keyboard layout, additional parameters).
    ///
    /// # Returns
    /// A reference to the created or retrieved session.
    pub fn get_or_create_x11_and_engine_session(&mut self, authenticated_session: AuthenticatedSession, session_config: SessionConfig, timeout: Duration) -> Result<String> {
        // Request display/session Id from WebX Session Manager
        let x11_session = self.x11_session_manager.get_or_create_x11_session(&authenticated_session, session_config.resolution().clone(), timeout)?;
        debug!("X11 session obtained for user \"{}\" on display \"{}\"", x11_session.account().username(), x11_session.display_id());

        self.get_or_create_engine_session(&x11_session, session_config)
    }

    fn get_or_create_engine_session(&mut self, x11_session: &X11Session, session_config: SessionConfig) -> Result<String> {
        // See if session already exists matching x11_session attributes
        if let Some(session) = self.sessions.iter().find(|session| 
            session.username() == x11_session.account().username() && 
            session.id() == x11_session.id() && 
            session.display_id() == x11_session.display_id()) {

            info!("Found existing Engine Session for user \"{}\" on display \"{}\" with id \"{}\"", session.username(), session.display_id(), session.id());
            return Ok(session.secret().to_string());
        }

        // Remove existing sessions for the user
        if let Some((index, session)) = self.sessions.iter_mut().enumerate().find(|(_, session)| session.username() == x11_session.account().username()) {
            debug!("Removing existing Engine Session for user \"{}\" on display \"{}\" with id \"{}\"", session.username(), session.display_id(), session.id());
            // stop the engine session
            session.stop_engine();

            // Remove the old engine session
            self.sessions.remove(index);        
        }

        // Create new session for the user
        self.create_engine_session(x11_session, None, &session_config)?;

        // Return the newly created session
        match self.sessions.iter().find(|session| session.username() == x11_session.account().username()) {
            Some(session) => Ok(session.secret().to_string()),
            None => Err(RouterError::EngineSessionError(format!("Could not retrieve Engine Session for user \"{}\"", x11_session.account().username())))
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
        let (index, session) = self.sessions.iter_mut().enumerate().find(|(_, session)| session.secret() == secret)
            .ok_or_else(|| RouterError::EngineSessionError(format!("Could not retrieve Engine Session by provided secret")))?;

        match self.engine_service.validate_engine(session.engine_mut(), 1) {
            Ok(_) => Ok(()),
            Err(error) => {
                // stop the engine session (if possible)
                session.stop_engine();

                // Remove the old engine session
                self.sessions.remove(index);   

                Err(error)
            }
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
        let session = self.sessions.iter_mut().find(|session| session.secret() == secret)
            .ok_or_else(|| RouterError::EngineSessionError(format!("Could not retrieve Engine Session with provided secret")))?;

        self.engine_service.send_engine_request(session.engine_mut(), request)
    }

    pub fn update_starting_processes(&mut self) {
        let all_sessions = self.x11_session_manager.sessions();

        // Clone creation processes so that we can alter the original vector
        let creation_processes_clone = self.creation_processes.clone();
        for process in creation_processes_clone.iter() {
            if let Some(x11_session) = all_sessions.iter().find(|session| session.id() == process.session_id()) {
                if x11_session.is_xorg_ready() {
                    // Start the window manager
                    info!("XorgCheckThread: Creating window manager for session id \"{}\" on display \"{}\"", x11_session.id(), x11_session.display_id());
                    if let Err(error) = self.x11_session_manager.create_window_manager(x11_session.id()) {
                        error!("XorgCheckThread: {}: removing creation process", error);
                        // Remove the creation process if the window manager creation fails
                        self.creation_processes.retain(|p| p.session_id() != process.session_id());
                    }

                    // Create the engine session
                    if let Err(error) = self.create_engine_session(x11_session, Some(process.secret().to_string()), process.session_config()) {
                        error!("XorgCheckThread: Failed to create engine session for user \"{}\" on display \"{}\" with id \"{}\": {}", 
                            x11_session.account().username(), 
                            x11_session.display_id(), 
                            x11_session.id(),
                            error);
                        // Remove the creation process if the engine session creation fails
                        self.creation_processes.retain(|p| p.session_id() != process.session_id());
                    } else {
                        info!("XorgCheckThread: Successfully created engine session for user \"{}\" on display \"{}\" with id \"{}\"", 
                            x11_session.account().username(), 
                            x11_session.display_id(), 
                            x11_session.id());
                        // Remove the creation process since the engine session was successfully created
                        self.creation_processes.retain(|p| p.session_id() != process.session_id());
                    }
                }
            } else {
                warn!("XorgCheckThread: No matching X11 session found for creation process with session id \"{}\". Removing process.", process.session_id());
                // Remove the creation process if no matching X11 session is found
                self.creation_processes.retain(|p| p.session_id() != process.session_id());
            }
        }

        // Get sessions that have no window manager yet byt have a ready Xorg but 
        let all_sessions = self.x11_session_manager.sessions();
        let ready_sessions: Vec<&X11Session> = all_sessions
            .iter()
            .filter(|session| session.window_manager().is_none())
            .filter(|session| session.is_xorg_ready())
            .collect();

        // For those that are ready, create the window manager
        for x11_session in ready_sessions {
            info!("XorgCheckThread: Creating window manager for session id \"{}\" on display \"{}\"", x11_session.id(), x11_session.display_id());
            if let Err(error) = self.x11_session_manager.create_window_manager(x11_session.id()) {
                error!("XorgCheckThread: {}", error);
            }
        }
    }

    /// Creates a new session for a user. This spawns a new WebX Engine process if necessary.
    ///
    /// # Arguments
    /// * `x11_session` - The X11 session details.
    /// * `session_config` - The session config (keyboard layout, additional parameters).
    ///
    /// # Returns
    /// A result indicating success or failure.
    fn create_engine_session(&mut self, x11_session: &X11Session, secret: Option<String>, session_config: &SessionConfig) -> Result<()> {
        info!("Creating Engine Session for user \"{}\" on display \"{}\" with id \"{}\"", &x11_session.account().username(), &x11_session.display_id(), x11_session.id());

        let secret = secret.unwrap_or(Uuid::new_v4().simple().to_string());

        // Spawn a new WebX Engine
        if let Some(engine) = self.multi_try_spawn_engine(&x11_session, &secret, session_config, 3) {

            let mut session = EngineSession::new(x11_session.account().username().to_string(), x11_session.display_id().to_string(), secret, engine);

            // Validate that the engine is running
            if let Err(error) = self.engine_service.validate_engine(session.engine_mut(), 3) {
                // Make sure the engine process has stopped
                session.stop_engine();
                return Err(RouterError::EngineSessionError(format!("Failed to validate that WebX Engine is running for user \"{}\" with session id \"{}\": {}", session.username(), session.id(), error)));
            }

            debug!("Created session with id \"{}\" on display \"{}\" for user \"{}\"", session.id(), session.display_id(), session.username());

            // Store session
            self.sessions.push(session);

            Ok(())
        } else {
            Err(RouterError::EngineSessionError(format!("Failed to launch WebX Engine for user \"{}\" with session id \"{}\"", x11_session.account().username(), x11_session.id())))
        }
    }

    /// Attempts to spawn a WebX Engine process multiple times until successful or the maximum number of tries is reached.
    ///
    /// # Arguments
    /// * `x11_session` - The X11 session details.
    /// * `session_config` - The session config (keyboard layout, additional parameters).
    /// * `tries` - The maximum number of attempts.
    ///
    /// # Returns
    /// * `Option<Engine>` - Some(Engine) if successful, None otherwise.
    fn multi_try_spawn_engine(&self, x11_session: &X11Session, secret: &str, session_config: &SessionConfig, tries: u64) -> Option<Engine> {
        let mut attempt = 0;
        while attempt < tries {
            debug!("Starting WebX Engine for user \"{}\" with session id \"{}\" on display \"{}\" (attempt {} / {})", x11_session.account().username(), x11_session.id(), x11_session.display_id(), attempt + 1, tries);
            match self.engine_service.spawn_engine(x11_session, secret, &self.context, &self.settings, session_config) {
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