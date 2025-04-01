/// The `X11Session` struct represents an X11 session, including its session ID,
/// username, display ID, and Xauthority file path.
/// The X11Session is returned from requests to the WebX Session Manager to create new X11 sessions.
pub struct X11Session {
    session_id: String,
    username: String,
    display_id: String,
    xauthority_file_path: String,
}

impl X11Session {
    /// Creates a new `X11Session` instance.
    ///
    /// # Arguments
    /// * `session_id` - The unique ID of the session.
    /// * `username` - The username associated with the session.
    /// * `display_id` - The display ID of the session.
    /// * `xauthority_file_path` - The path to the Xauthority file.
    ///
    /// # Returns
    /// A new `X11Session` instance.
    pub fn new(session_id: String, username: String, display_id: String, xauthority_file_path: String) -> Self {
        Self {
            session_id,
            username,
            display_id,
            xauthority_file_path,
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

    /// Retrieves the path to the Xauthority file.
    pub fn xauthority_file_path(&self) -> &str {
        &self.xauthority_file_path
    }
}

