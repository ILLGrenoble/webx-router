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

    pub fn get_existing_session(&self, x11_session: &X11Session) -> Option<&Session> {
        self.sessions.iter().find(|session| session.username() == x11_session.username() && session.id() == x11_session.session_id() && session.display_id() == x11_session.display_id())
    }

    pub fn stop_sessions(&mut self) {
        for session in self.sessions.iter_mut() {
            session.stop();
        }

        self.sessions.clear();
    }

    pub fn remove_previous_session_for_user(&mut self, username: &str) {
        if let Some(session) = self.sessions.iter_mut().find(|session| session.username() == username) {
            session.stop();
        }

        if let Some(index) = self.sessions.iter().position(|a_session| a_session.username() == username) {
            self.sessions.remove(index);        
        }
    }
}