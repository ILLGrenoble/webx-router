use super::Engine;
use crate::common::{System, random_string};
use crate::sesman::X11Session;

/// The `EngineSession` struct represents a user session, including its X11 session and WebX Engine.
pub struct EngineSession {
    secret: String,
    x11_session: X11Session,
    engine: Engine,
    last_activity: u64,
}

impl EngineSession {
    /// Creates a new `EngineSession` instance.
    ///
    /// # Arguments
    /// * `x11_session` - The X11 session details.
    /// * `engine` - The WebX Engine instance.
    pub fn new(x11_session: X11Session, engine: Engine) -> Self {
        Self {
            secret: random_string(32),
            x11_session,
            engine,
            last_activity: System::current_time_s()
        }
    }

    /// Checks if the session is active based on the inactivity timeout.
    ///
    /// # Arguments
    /// * `session_inactivity_s` - The inactivity timeout in seconds.
    ///
    /// # Returns
    /// `true` if the session is active, `false` otherwise.
    pub fn is_active(&self, session_inactivity_s: u64) -> bool {
        let current_time = System::current_time_s();
        current_time - self.last_activity <= session_inactivity_s
    }

    /// Updates the activity timestamp of the session.
    pub fn update_activity(&mut self) {
        let current_time = System::current_time_s();
        trace!("Updating activity of session {} to {}", self.id(), current_time);
        self.last_activity = current_time;
    }

    /// Retrieves the session secret.
    pub fn secret(&self) -> &str {
        &self.secret
    }

    /// Retrieves the session ID.
    pub fn id(&self) -> &str {
        return &self.x11_session.id();
    }

    /// Retrieves the display ID of the session.
    pub fn display_id(&self) -> &str {
        return &self.x11_session.display_id();
    }

    /// Retrieves the username associated with the session.
    pub fn username(&self) -> &str {
        return &self.x11_session.account().username();
    }

    /// Retrieves the mutable WebX Engine instance associated with the session.
    pub fn engine_mut(&mut self) -> &mut Engine {
        return &mut self.engine;
    }

    /// Stops the session and cleans up resources.
    pub fn stop_engine(&mut self) {
        debug!("Stopping WebX Engine for \"{}\" on display \"{}\" with id \"{}\"", self.username(), self.display_id(), self.id());
        match self.engine.close() {
            Ok(_) => {
                debug!("Stopped WebX Engine for \"{}\" on display \"{}\" with id \"{}\"", self.username(), self.display_id(), self.id());
            },
            Err(error) => error!("Failed to stop WebX Engine for \"{}\": {}", self.username(), error),
        }

    }

}