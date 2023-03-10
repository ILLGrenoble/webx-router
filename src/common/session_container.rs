use crate::common::{Session, X11Session};

pub struct SessionContainer {
    sessions: Vec<Session>,
}

impl SessionContainer {

    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
        }
    }

    pub fn add_session(&mut self, session: Session) {
        self.sessions.push(session);
    }

    pub fn get_session_by_username(&self, username: &str) -> Option<&Session> {
        self.sessions.iter().find(|session| session.username() == username)
    }

    pub fn get_session_by_session_id(&self, session_id: &str) -> Option<&Session> {
        self.sessions.iter().find(|session| session.id() == session_id)
    }

    pub fn get_mut_session_by_session_id(&mut self, session_id: &str) -> Option<&mut Session> {
        self.sessions.iter_mut().find(|session| session.id() == session_id)
    }

    pub fn get_session_by_x11session(&self, x11_session: &X11Session) -> Option<&Session> {
        self.sessions.iter().find(|session| session.username() == x11_session.username() && session.id() == x11_session.session_id() && session.display_id() == x11_session.display_id())
    }

    pub fn stop_sessions(&mut self) {
        for session in self.sessions.iter_mut() {
            session.stop();
        }

        self.sessions.clear();
    }

    pub fn remove_session_for_user(&mut self, username: &str) {
        if let Some(session) = self.sessions.iter_mut().find(|session| session.username() == username) {
            session.stop();
        }

        if let Some(index) = self.sessions.iter().position(|a_session| a_session.username() == username) {
            self.sessions.remove(index);        
        }
    }

    pub fn remove_session_with_id(&mut self, session_id: &str) {
        if let Some(session) = self.sessions.iter_mut().find(|session| session.id() == session_id) {
            session.stop();
        }

        if let Some(index) = self.sessions.iter().position(|a_session| a_session.id() == session_id) {
            self.sessions.remove(index);        
        }
    }

    pub fn get_inactive_session_ids(&self, session_inactivity_s: u64) -> Vec<(String, String)> {
        self.sessions
            .iter()
            .filter(|session| !session.is_active(session_inactivity_s))
            .map(|session| (session.id().to_string(), session.username().to_string()))
            .collect()
    }
}