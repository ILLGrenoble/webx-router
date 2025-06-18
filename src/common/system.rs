use std::time::{SystemTime, UNIX_EPOCH};
use nix::unistd::User;

/// The `System` struct provides utility methods for system-related operations,
/// such as retrieving the current time and the current username.
pub struct System {
}

impl System {
    /// Retrieves the current time in seconds since the UNIX epoch.
    ///
    /// # Returns
    /// The current time in seconds as a `u64`.
    pub fn current_time_s() -> u64 {
        if let Ok(current_time) = SystemTime::now().duration_since(UNIX_EPOCH) {
            current_time.as_secs()
     
        } else {
            0
        }
    }

    pub fn get_user(username: &str) -> Option<User> {
        if let Ok(Some(user)) = User::from_name(username) {
            Some(user)
        } else {
            error!("User {} not found", username);
            None
        }
    }
}