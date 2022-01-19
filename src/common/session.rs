use crate::common::{Engine, X11Session};

use signal_child::Signalable;

pub struct Session {
    x11_session: X11Session,
    engine: Engine,
}

impl Session {

    pub fn new(x11_session: X11Session, engine: Engine) -> Self {
        Self {
            x11_session,
            engine,
        }
    }

    pub fn id(&self) -> &str {
        return &self.x11_session.session_id();
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

    pub fn stop(&mut self) {
        let engine = self.engine();
        let process = engine.process();
        let process_id = { process.id() };
        match process.interrupt() {
            Ok(_) => {
                debug!("Shutdown WebX Engine for {} on display {}", self.username(), self.display_id());
            },
            Err(error) => error!("Failed to interrupt WebX Engine for {} running on PID {}: {}", self.username(), process_id, error),
        }

    }

}