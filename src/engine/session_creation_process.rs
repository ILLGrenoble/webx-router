use super::SessionConfig;

#[derive(Clone)]
pub struct SessionCreationProcess {
    session_id: String,
    username: String,
    display_id: String,
    session_config: SessionConfig,
    secret: String,
}

impl SessionCreationProcess {
    /// Creates a new `SessionCreationProcess` instance.
    ///
    /// # Arguments
    /// * `session_id` - The unique identifier for the session.
    /// * `username` - The username of the session owner.
    /// * `display_id` - The X11 display ID associated with the session.
    /// * `session_config` - The configuration for the session.
    /// * `secret` - The session secret (this is the session_id inside the webx-engine).
    pub fn new(session_id: String, username: String, display_id: String, session_config: SessionConfig, secret: String) -> Self {
        Self {
            session_id,
            username,
            display_id,
            session_config,
            secret,
        }
    }

    /// Retrieves the session ID.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Retrieves the username associated with the session.
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Retrieves the display ID of the session.
    pub fn display_id(&self) -> &str {
        &self.display_id
    }

    /// Retrieves the session configuration.
    pub fn session_config(&self) -> &SessionConfig {
        &self.session_config
    }

    /// Retrieves the session secret.
    pub fn secret(&self) -> &str {
        &self.secret
    }
}