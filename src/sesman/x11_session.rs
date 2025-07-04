use crate::common::{ProcessHandle};
use crate::authentication::Account;
use super::{ScreenResolution};

/// The `Session` struct represents a user session managed by the WebX Session Manager.
/// It contains details about the session, such as the user, session ID, the Xorg process and the Window Manager process.
#[derive(Clone)]
pub struct X11Session {
    id: String,
    account: Account,
    display_id: String,
    xauthority_file_path: String,
    xorg: ProcessHandle,
    window_manager: Option<ProcessHandle>,
    resolution: ScreenResolution,
}

#[allow(dead_code)]
impl X11Session {
    /// Creates a new `Session` instance.
    ///
    /// # Arguments
    /// * `id` - The unique identifier for the session.
    /// * `username` - The username of the session owner.
    /// * `uid` - The user ID of the session owner.
    /// * `display_id` - The X11 display ID.
    /// * `xauthority_file_path` - The path to the Xauthority file.
    /// * `xorg` - The process handle for the Xorg server.
    /// * `resolution` - The screen resolution for the session.
    ///
    /// # Returns
    /// A new `Session` instance.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        account: Account,
        display_id: String,
        xauthority_file_path: String,
        xorg: ProcessHandle,
        resolution: ScreenResolution,
    ) -> Self {
        Self {
            id,
            account,
            display_id,
            xauthority_file_path,
            xorg,
            window_manager: None,
            resolution,
        }
    }

    /// Returns the unique identifier for the session.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the acount of the session owner.
    pub fn account(&self) -> &Account {
        &self.account
    }

    /// Returns the X11 display ID.
    pub fn display_id(&self) -> &str {
        &self.display_id
    }

    /// Returns the path to the Xauthority file.
    pub fn xauthority_file_path(&self) -> &str {
        &self.xauthority_file_path
    }

    /// Returns the process handle for the Xorg server.
    pub fn xorg(&self) -> &ProcessHandle {
        &self.xorg
    }

    /// Returns the option process handle for the window manager.
    pub fn window_manager(&self) -> &Option<ProcessHandle> {
        &self.window_manager
    }

    /// Sets the process handle for the window manager.
    pub fn set_window_manager(&mut self, window_manager: ProcessHandle) {
        self.window_manager = Some(window_manager);
    }

    /// Returns the screen resolution for the session.
    pub fn resolution(&self) -> &ScreenResolution {
        &self.resolution
    }
}
