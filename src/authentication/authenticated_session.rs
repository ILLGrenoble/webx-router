use super::Account;
use pam_client::env_list::EnvList;
use std::ffi::OsString;

/// The `AuthenticatedSession` struct represents a user session that has been authenticated.
/// It contains the account associated with the session and the environment variables for the session.
#[derive(Clone)]
pub struct AuthenticatedSession {
    account: Account,
    environment: Vec<(OsString, OsString)>,
}

impl AuthenticatedSession {
    /// Creates a new `AuthenticatedSession` instance.
    ///
    /// # Arguments
    /// * `account` - The account associated with the session.
    /// * `environment` - The environment variables for the session.
    ///
    /// # Returns
    /// A new `AuthenticatedSession` instance.
    pub fn new(account: Account, environment: EnvList) -> Self {
        Self { account, environment: environment.into() }
    }

    /// Returns the account associated with the session.
    pub fn account(&self) -> &Account {
        &self.account
    }

    /// Returns the environment variables for the session.
    pub fn environment(&self) -> &Vec<(OsString, OsString)> {
        &self.environment
    }
}