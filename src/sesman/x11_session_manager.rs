use nix::unistd::User;
use uuid::Uuid;

use crate::{
    authentication::{Authenticator, Credentials},
    common::{RouterError, Result},
};

use super::{XorgService, Account, X11Session, ScreenResolution};

/// The `X11SessionManager` struct provides functionality for managing user sessions,
/// including creating, retrieving, and terminating sessions.
pub struct X11SessionManager {
    authenticator: Authenticator,
    xorg_service: XorgService,
}

impl X11SessionManager {
    /// Creates a new `X11SessionManager` instance.
    ///
    /// # Arguments
    /// * `authenticator` - The authenticator for user authentication.
    /// * `xorg_service` - The Xorg service for managing Xorg sessions.
    ///
    /// # Returns
    /// A new `X11SessionManager` instance.
    pub fn new(authenticator: Authenticator, 
               xorg_service: XorgService
    ) -> Self {
        Self {
            authenticator,
            xorg_service,
        }
    }

    /// Creates a new session for a user.
    ///
    /// # Arguments
    /// * `credentials` - The user's credentials.
    /// * `resolution` - The screen resolution for the session.
    ///
    /// # Returns
    /// A `Result` containing the created `Session` or an `ApplicationError`.
    pub fn create_session(&self, credentials: &Credentials, resolution: ScreenResolution) -> Result<X11Session> {
        return match self.authenticator.authenticate(credentials) {
            Ok(environment) => {
                debug!("Successfully authenticated user: {}", &credentials.username());
                if let Ok(Some(user)) = User::from_name(credentials.username()) {
                    debug!("Found user: {}", &credentials.username());
                    if let Some(account) = Account::from_user(user) {

                        // if the user already has an x session running then exit early...
                        if let Some(session) = self.xorg_service.get_session_for_user(account.uid()) {
                            debug!("User {} already has a session {}", &credentials.username(), session.id());
                            return Ok(session);
                        }

                        let webx_user = User::from_name("webx").unwrap().unwrap();
                        // create the necessary configuration files
                        if let Err(error) = self.xorg_service.create_user_files(&account, &webx_user) {
                            return Err(RouterError::X11SessionError(format!("Error occurred setting up the configuration for a session {}", error)));
                        }

                        // finally, let's launch the x server...
                        return self.xorg_service.execute(&account, &webx_user, resolution, environment);
                    }
                    return Err(RouterError::X11SessionError(format!("User {} is invalid. check they have a home directory?", credentials.username())));
                }
                Err(RouterError::X11SessionError(format!("Could not find user {}", credentials.username())))
            }
            Err(error) => {
                Err(RouterError::X11SessionError(format!("Error authenticating user {}", error)))
            }
        }
    }

    /// Retrieves all active sessions.
    ///
    /// # Returns
    /// An `Option` containing a vector of `Session` instances, or `None` if no sessions are found.
    pub fn get_all(&self) -> Option<Vec<X11Session>> {
        self.xorg_service.get_all_sessions()
    }

    /// Terminates a session by its unique identifier.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the session to terminate.
    ///
    /// # Returns
    /// A `Result` indicating success or an `ApplicationError`.
    pub fn kill_by_id(&self, id: &Uuid) -> Result<()> {
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
    /// A `Result` indicating success or an `ApplicationError`.
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

    fn kill_session(&self, session: &X11Session) -> Result<()> {
        debug!("Killing window manager on display {} with pid: {}", session.display_id(), session.window_manager().pid());
        session.window_manager().kill()?;

        debug!("Killing Xorg on display {} with pid: {}", session.display_id(), session.xorg().pid());
        session.xorg().kill()?;

        self.xorg_service.remove_session(session);

        Ok(())
    }

    /// Cleans up zombie sessions by removing sessions whose processes are no longer running.
    pub fn clean_up(&self) {
        if self.xorg_service.clean_up() > 0 {
            info!("Cleaned up {} zombie sessions", self.xorg_service.clean_up());
        }
    }
}
