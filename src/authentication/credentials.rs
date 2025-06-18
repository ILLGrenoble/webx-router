/// The `Credentials` struct represents a user's login credentials, including their username and password.
pub struct Credentials {
    username: String,
    password: String,
}

impl Credentials {
    /// Creates a new `Credentials` instance.
    ///
    /// # Arguments
    /// * `username` - The username of the user.
    /// * `password` - The password of the user.
    ///
    /// # Returns
    /// A new `Credentials` instance.
    pub fn new(username: String, password: String) -> Self {
        Credentials { username, password }
    }

    /// Returns the username of the user.
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Returns the password of the user.
    pub fn password(&self) -> &str {
        &self.password
    }
}
