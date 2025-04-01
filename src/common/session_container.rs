use crate::common::{Session, X11Session};

/// The `SessionContainer` struct manages a collection of active sessions.
/// It provides methods to add, retrieve, update, and remove sessions.
pub struct SessionContainer {
    sessions: Vec<Session>,
}

impl SessionContainer {
    /// Creates a new `SessionContainer` instance.
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
        }
    }

    /// Adds a new session to the container.
    ///
    /// # Arguments
    /// * `session` - The session to add.
    pub fn add_session(&mut self, session: Session) {
        self.sessions.push(session);
    }

    /// Retrieves a session by username.
    ///
    /// # Arguments
    /// * `username` - The username associated with the session.
    ///
    /// # Returns
    /// An optional reference to the session.
    pub fn get_session_by_username(&self, username: &str) -> Option<&Session> {
        self.sessions.iter().find(|session| session.username() == username)
    }

    /// Retrieves a session by session ID.
    ///
    /// # Arguments
    /// * `session_id` - The ID of the session.
    ///
    /// # Returns
    /// An optional reference to the session.
    pub fn get_session_by_session_id(&self, session_id: &str) -> Option<&Session> {
        self.sessions.iter().find(|session| session.id() == session_id)
    }

    /// Retrieves a mutable reference to a session by session ID.
    ///
    /// # Arguments
    /// * `session_id` - The ID of the session.
    ///
    /// # Returns
    /// An optional mutable reference to the session.
    pub fn get_mut_session_by_session_id(&mut self, session_id: &str) -> Option<&mut Session> {
        self.sessions.iter_mut().find(|session| session.id() == session_id)
    }

    /// Retrieves a session by its X11 session details.
    ///
    /// # Arguments
    /// * `x11_session` - The X11 session details.
    ///
    /// # Returns
    /// An optional reference to the session.
    pub fn get_session_by_x11session(&self, x11_session: &X11Session) -> Option<&Session> {
        self.sessions.iter().find(|session| session.username() == x11_session.username() && session.id() == x11_session.session_id() && session.display_id() == x11_session.display_id())
    }

    /// Stops all active sessions and clears the container.
    pub fn stop_sessions(&mut self) {
        for session in self.sessions.iter_mut() {
            session.stop();
        }

        self.sessions.clear();
    }

    /// Removes a session for a specific user.
    ///
    /// # Arguments
    /// * `username` - The username associated with the session to remove.
    pub fn remove_session_for_user(&mut self, username: &str) {
        if let Some(session) = self.sessions.iter_mut().find(|session| session.username() == username) {
            session.stop();
        }

        if let Some(index) = self.sessions.iter().position(|a_session| a_session.username() == username) {
            self.sessions.remove(index);        
        }
    }

    /// Removes a session by its session ID.
    ///
    /// # Arguments
    /// * `session_id` - The ID of the session to remove.
    pub fn remove_session_with_id(&mut self, session_id: &str) {
        if let Some(session) = self.sessions.iter_mut().find(|session| session.id() == session_id) {
            session.stop();
        }

        if let Some(index) = self.sessions.iter().position(|a_session| a_session.id() == session_id) {
            self.sessions.remove(index);        
        }
    }

    /// Retrieves the IDs of inactive sessions based on the inactivity timeout.
    ///
    /// # Arguments
    /// * `session_inactivity_s` - The inactivity timeout in seconds.
    ///
    /// # Returns
    /// A vector of tuples containing session IDs and usernames of inactive sessions.
    pub fn get_inactive_session_ids(&self, session_inactivity_s: u64) -> Vec<(String, String)> {
        self.sessions
            .iter()
            .filter(|session| !session.is_active(session_inactivity_s))
            .map(|session| (session.id().to_string(), session.username().to_string()))
            .collect()
    }
}