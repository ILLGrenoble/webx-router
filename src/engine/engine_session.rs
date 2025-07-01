use super::Engine;
use crate::sesman::X11Session;

/// The `EngineSession` struct represents a user session, including its X11 session and WebX Engine.
pub struct EngineSession {
    secret: String,
    x11_session: X11Session,
    engine: Engine,
}

impl EngineSession {
    /// Creates a new `EngineSession` instance.
    ///
    /// # Arguments
    /// * `secret` - The session secret (this is the session_id inside the webx-engine)
    /// * `x11_session` - The X11 session details.
    /// * `engine` - The WebX Engine instance.
    pub fn new(secret: String, x11_session: X11Session, engine: Engine) -> Self {
        Self {
            secret,
            x11_session,
            engine,
        }
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
                info!("Stopped WebX Engine for \"{}\" on display \"{}\" with id \"{}\"", self.username(), self.display_id(), self.id());
            },
            Err(error) => error!("Failed to stop WebX Engine for \"{}\": {}", self.username(), error),
        }

    }

}