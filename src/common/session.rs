use std::fs;

use crate::common::{Engine, X11Session, System};

use signal_child::Signalable;

pub struct Session {
    x11_session: X11Session,
    engine: Engine,
    last_activity: u64,
}

impl Session {

    pub fn new(x11_session: X11Session, engine: Engine) -> Self {
        Self {
            x11_session,
            engine,
            last_activity: System::current_time_s()
        }
    }

    pub fn is_active(&self, session_inactivity_s: u64) -> bool {
        let current_time = System::current_time_s();
        current_time - self.last_activity <= session_inactivity_s
    }

    pub fn update_activity(&mut self) {
        let current_time = System::current_time_s();
        trace!("Updating activity of session {} to {}", self.id(), current_time);
        self.last_activity = current_time;
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