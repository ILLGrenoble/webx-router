use std::fs;

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

    pub fn engine(&self) -> &Engine {
        return &self.engine;
    }

    pub fn stop(&mut self) {
        let ipc_path = self.engine.ipc().to_string();

        let process = self.engine.process();
        let process_id = { process.id() };
        match process.interrupt() {
            Ok(_) => {
                if let Err(error) = process.wait() {
                    warn!("Failed to wait for WebX Engine for {} running on PID {} to terminate: {}", self.username(), process_id, error);

                } else {
                    debug!("Shutdown WebX Engine for {} on display {}", self.username(), self.display_id());

                    // Delete the IPC socket file
                    fs::remove_file(ipc_path).unwrap();
                }
            },
            Err(error) => error!("Failed to interrupt WebX Engine for {} running on PID {}: {}", self.username(), process_id, error),
        }

    }

}