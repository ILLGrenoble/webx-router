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
    /// * `u64` - The current time in seconds since the UNIX epoch. Returns 0 if the system time cannot be determined.
    pub fn current_time_s() -> u64 {
        if let Ok(current_time) = SystemTime::now().duration_since(UNIX_EPOCH) {
            current_time.as_secs()
        } else {
            0
        }
    }

    /// Retrieves a `User` struct for the specified username.
    ///
    /// # Arguments
    /// * `username` - The username to look up.
    ///
    /// # Returns
    /// * `Option<User>` - Some(User) if the user exists, or None if not found or an error occurs.
    pub fn get_user(username: &str) -> Option<User> {
        if let Ok(Some(user)) = User::from_name(username) {
            Some(user)
        } else {
            error!("User {} not found", username);
            None
        }
    }
}