use super::Engine;

/// The `EngineSession` struct represents a user session, including its X11 session and WebX Engine.
pub struct EngineSession {
    username: String,
    display_id: String,
    secret: String,
    engine: Engine,
}

impl EngineSession {
    /// Creates a new `EngineSession` instance.
    ///
    /// # Arguments
    /// * `username` - The username of the session owner.
    /// * `display_id` - The X11 display ID associated with the session.
    /// * `secret` - The session secret (this is the session_id inside the webx-engine)
    /// * `engine` - The WebX Engine instance.
    pub fn new(username: String, display_id: String, secret: String, engine: Engine) -> Self {
        Self {
            username,
            display_id,
            secret,
            engine,
        }
    }

    /// Retrieves the session secret.
    pub fn secret(&self) -> &str {
        &self.secret
    }

    /// Retrieves the session ID.
    pub fn id(&self) -> &str {
        self.engine.session_id()
    }

    /// Retrieves the display ID of the session.
    pub fn display_id(&self) -> &str {
        &self.display_id
    }

    /// Retrieves the username associated with the session.
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Retrieves the mutable WebX Engine instance associated with the session.
    pub fn engine_mut(&mut self) -> &mut Engine {
        return &mut self.engine;
    }

    /// Stops the session and cleans up resources.
    pub fn stop_engine(&mut self) {
        debug!("Stopping WebX Engine for \"{}\" on display \"{}\" with id \"{}\"", self.username, self.display_id, self.id());
        match self.engine.close() {
            Ok(_) => {
                info!("Stopped WebX Engine for \"{}\" on display \"{}\" with id \"{}\"", self.username, self.display_id, self.id());
            },
            Err(error) => error!("Failed to stop WebX Engine for \"{}\": {}", self.username, error),
        }

    }

}