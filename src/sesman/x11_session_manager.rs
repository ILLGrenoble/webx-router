use std::{thread, time};
use std::sync::Mutex;

use crate::{
    authentication::AuthenticatedSession,
    common::{RouterError, Result, SesManSettings},
};

use super::{XorgService, X11Session, ScreenResolution};

/// The `X11SessionManager` struct provides functionality for managing user X11 sessions,
/// including creating, retrieving, and terminating sessions.
pub struct X11SessionManager {
    xorg_service: XorgService,
    sessions: Mutex<Vec<X11Session>>,
}

impl X11SessionManager {
    /// Creates a new `X11SessionManager` instance.
    ///
    /// # Arguments
    /// * `settings` - The session manager settings.
    ///
    /// # Returns
    /// A new `X11SessionManager` instance.
    pub fn new(settings: &SesManSettings) -> Self {
        Self {
            xorg_service: XorgService::new(settings.xorg.to_owned()),
            sessions: Mutex::new(Vec::new()),
        }
    }

    /// Creates a new session for a user.
    ///
    /// # Arguments
    /// * `authenticated_session` - The authenticated user session (account and environment).
    /// * `resolution` - The screen resolution for the session.
    ///
    /// # Returns
    /// A `Result` containing the created `X11Session` or a `RouterError`.
    pub fn create_session_async(&self, authenticated_session: &AuthenticatedSession, resolution: ScreenResolution) -> Result<X11Session> {

        // let's launch the x server...
        return self.xorg_service.create_xorg(authenticated_session, resolution);
    }

    /// Creates a new session for a user.
    ///
    /// # Arguments
    /// * `authenticated_session` - The authenticated user session (account and environment).
    /// * `resolution` - The screen resolution for the session.
    ///
    /// # Returns
    /// A `Result` containing the created `X11Session` or a `RouterError`.
    pub fn create_session(&self, authenticated_session: &AuthenticatedSession, resolution: ScreenResolution) -> Result<X11Session> {
        let x11_session = self.create_xorg(authenticated_session, resolution)?;

        // Release the lock on sessions before sleeping
        // Sleep for 1 second (wait for x server to start)
        thread::sleep(time::Duration::from_millis(1000));

        let x11_session = self.create_window_manager(x11_session.id())?;
        
        let wm_pid = match x11_session.window_manager() {
            Some(wm) => wm.pid().to_string(),
            None => "<None>".to_string(),
        };
        info!("Started Xorg on display \"{}\" with process id {} and window manager process id {}", x11_session.display_id(), x11_session.xorg().pid(), wm_pid);

        Ok(x11_session)
    }
    
    fn create_xorg(&self, authenticated_session: &AuthenticatedSession, resolution: ScreenResolution) -> Result<X11Session> {
        if let Ok(mut sessions) = self.sessions.lock() {
            // if the user already has an x session running then exit early...
            if let Some(session) = sessions.iter().find(|session| session.account().uid() == authenticated_session.account().uid()) {
                debug!("User {} already has a session {}", &authenticated_session.account().username(), session.id());
                return Ok(session.clone());
            }

            // let's launch the x server...
            let x11_session = self.xorg_service.create_xorg(authenticated_session, resolution)?;

            sessions.push(x11_session.clone());

            Ok(x11_session)

        } else {
            return Err(RouterError::X11SessionError("Failed to lock sessions".to_string()));
        }
    }

    fn create_window_manager(&self, session_id: &str) -> Result<X11Session> {
        if let Ok(mut sessions) = self.sessions.lock() {
            // Verify that X11 session exists
            let x11_session = sessions.iter_mut().find(|session| session.id() == session_id)
                .ok_or_else(|| RouterError::X11SessionError(format!("X11 Session no longer exists when spawning Window Manager process")))?;

            let window_manager = self.xorg_service.create_window_manager(&x11_session)?;

            x11_session.set_window_manager(window_manager);

            Ok(x11_session.clone())

        } else {
            Err(RouterError::X11SessionError("Failed to lock sessions".to_string()))
        }
    }

    /// Retrieves all active X11 sessions.
    ///
    /// # Returns
    /// An `Option` containing a vector of `X11Session` instances, or `None` if no sessions are found.
    pub fn get_all(&self) -> Option<Vec<X11Session>> {
        if let Ok(sessions) = self.sessions.lock() {
            return Some(sessions.to_vec());
        }
        None
    }

    /// Terminates all active sessions.
    ///
    /// # Returns
    /// A `Result` indicating success or a `RouterError`.
    pub fn kill_all(&self) -> Result<()> {
        if let Ok(sessions) = self.sessions.lock() {
            for session in sessions.iter() {
                self.kill_session(&session)?;
            } 
            Ok(())
        
        } else {
            Err(RouterError::X11SessionError("Failed to lock sessions".to_string()))
        }
    }

    /// Terminates a specific session by killing its window manager and Xorg processes,
    /// and removing it from the session list.
    ///
    /// # Arguments
    /// * `session` - The session to terminate.
    ///
    /// # Returns
    /// A `Result` indicating success or a `RouterError`.
    fn kill_session(&self, session: &X11Session) -> Result<()> {
        if let Ok(mut sessions) = self.sessions.lock() {
            if let Some(window_manager) = session.window_manager() {
                debug!("Killing window manager on display {} with pid: {}", session.display_id(), window_manager.pid());
                window_manager.kill()?;
            }
            
            debug!("Killing Xorg on display {} with pid: {}", session.display_id(), session.xorg().pid());
            session.xorg().kill()?;

            // Remove the session from the active sessions list
            sessions.retain(|s| s.id() != session.id());

            info!("Stopped Xorg and Window Manager processes on display \"{}\" with id \"{}\"", session.display_id(), session.id());

            Ok(())

        } else {
            Err(RouterError::X11SessionError("Failed to lock sessions".to_string()))
        }
    }

}