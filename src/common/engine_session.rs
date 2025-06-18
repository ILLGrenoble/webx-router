use std::fs;
use uuid::Uuid;

use crate::common::{Engine, System};
use crate::sesman::{X11Session};

use signal_child::Signalable;

/// The `EngineSession` struct represents a user session, including its X11 session and WebX Engine.
pub struct EngineSession {
    x11_session: X11Session,
    engine: Engine,
    last_activity: u64,
}

impl EngineSession {
    /// Creates a new `EngineSession` instance.
    ///
    /// # Arguments
    /// * `x11_session` - The X11 session details.
    /// * `engine` - The WebX Engine instance.
    pub fn new(x11_session: X11Session, engine: Engine) -> Self {
        Self {
            x11_session,
            engine,
            last_activity: System::current_time_s()
        }
    }

    /// Checks if the session is active based on the inactivity timeout.
    ///
    /// # Arguments
    /// * `session_inactivity_s` - The inactivity timeout in seconds.
    ///
    /// # Returns
    /// `true` if the session is active, `false` otherwise.
    pub fn is_active(&self, session_inactivity_s: u64) -> bool {
        let current_time = System::current_time_s();
        current_time - self.last_activity <= session_inactivity_s
    }

    /// Updates the activity timestamp of the session.
    pub fn update_activity(&mut self) {
        let current_time = System::current_time_s();
        trace!("Updating activity of session {} to {}", self.id(), current_time);
        self.last_activity = current_time;
    }

    /// Retrieves the session ID.
    pub fn id(&self) -> &Uuid {
        return &self.x11_session.id();
    }

    /// Retrieves the display ID of the session.
    pub fn display_id(&self) -> &str {
        return &self.x11_session.display_id();
    }

    /// Retrieves the username associated with the session.
    pub fn username(&self) -> &str {
        return &self.x11_session.account().username();
    }

    /// Retrieves the WebX Engine instance associated with the session.
    pub fn engine(&self) -> &Engine {
        return &self.engine;
    }

    /// Stops the session and cleans up resources.
    pub fn stop_engine(&mut self) {
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
                    let _ = fs::remove_file(ipc_path);
                }
            },
            Err(error) => error!("Failed to interrupt WebX Engine for {} running on PID {}: {}", self.username(), process_id, error),
        }

    }

}