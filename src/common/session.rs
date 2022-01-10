use crate::common::{Engine, X11Session};
use uuid::Uuid;

pub struct Session {
    id: Uuid,
    x11_session: X11Session,
    engine: Engine,
}

impl Session {

    pub fn new(id: Uuid, x11_session: X11Session, engine: Engine) -> Self {
        Self {
            id,
            x11_session,
            engine,
        }
    }

    pub fn id(&self) -> &Uuid {
        return &self.id;
    }

    pub fn display_id(&self) -> &str {
        return &self.x11_session.display_id();
    }

    pub fn username(&self) -> &str {
        return &self.x11_session.username();
    }

    pub fn engine(&mut self) -> &mut Engine {
        return &mut self.engine;
    }

}