use nix::unistd::User;

use crate::{
    authentication::AuthenticatedSession,
    common::{RouterError, Result, SesManSettings},
};

use super::{XorgService, X11Session, ScreenResolution};

/// The `X11SessionManager` struct provides functionality for managing user X11 sessions,
/// including creating, retrieving, and terminating sessions.
pub struct X11SessionManager {
    xorg_service: XorgService,
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
    pub fn create_session(&self, authenticated_session: &AuthenticatedSession, resolution: ScreenResolution) -> Result<X11Session> {
        // if the user already has an x session running then exit early...
        if let Some(session) = self.xorg_service.get_session_for_user(authenticated_session.account().uid()) {
            debug!("User {} already has a session {}", &authenticated_session.account().username(), session.id());
            return Ok(session);
        }

        let webx_user = User::from_name("webx").unwrap().unwrap();
        // create the necessary configuration files
        if let Err(error) = self.xorg_service.create_user_files(authenticated_session.account(), &webx_user) {
            return Err(RouterError::X11SessionError(format!("Error occurred setting up the configuration for a session {}", error)));
        }

        // finally, let's launch the x server...
        return self.xorg_service.execute(authenticated_session.account(), &webx_user, resolution, authenticated_session.environment());
    }

    /// Retrieves all active X11 sessions.
    ///
    /// # Returns
    /// An `Option` containing a vector of `X11Session` instances, or `None` if no sessions are found.
    pub fn get_all(&self) -> Option<Vec<X11Session>> {
        self.xorg_service.get_all_sessions()
    }

    /// Terminates a session by its unique identifier.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the session to terminate.
    ///
    /// # Returns
    /// A `Result` indicating success or a `RouterError`.
    pub fn kill_by_id(&self, id: &str) -> Result<()> {
        if let Some(session) = self.xorg_service.get_by_id(id) {
            // kill the processes
            // the session will be automatically removed by the clean up procedure
            self.kill_session(&session)?;

            return Ok(());
        }
        Err(RouterError::X11SessionError(format!("Session {} not found", id)))
    }

    /// Terminates all active sessions.
    ///
    /// # Returns
    /// A `Result` indicating success or a `RouterError`.
    pub fn kill_all(&self) -> Result<()> {
        if let Some(sessions) = self.xorg_service.get_all_sessions() {
            for session in sessions {
                self.kill_session(&session)?;
            } 
        }

        // Remove all zombie sessions
        self.clean_up();

        Ok(())
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
        debug!("Killing window manager on display {} with pid: {}", session.display_id(), session.window_manager().pid());
        session.window_manager().kill()?;

        debug!("Killing Xorg on display {} with pid: {}", session.display_id(), session.xorg().pid());
        session.xorg().kill()?;

        self.xorg_service.remove_session(session);

        info!("Stopped Xorg and Window Manager processes on display \"{}\" with id \"{}\"", session.display_id(), session.id());

        Ok(())
    }

    /// Cleans up zombie sessions by removing sessions whose processes are no longer running.
    pub fn clean_up(&self) {
        if self.xorg_service.clean_up() > 0 {
            info!("Cleaned up {} zombie sessions", self.xorg_service.clean_up());
        }
    }
}