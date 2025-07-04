use pam_client::env_list::EnvList;
use pam_client::{Context, Flag};
use pam_client::conv_mock::Conversation;
use nix::unistd::User;

use crate::authentication::Credentials;
use crate::common::{Result, RouterError};
use super::{Account, AuthenticatedSession};

/// The `Authenticator` struct provides functionality for authenticating users using PAM (Pluggable Authentication Modules).
pub struct Authenticator {
    service: String,
}

impl Authenticator {
    /// Creates a new `Authenticator` instance.
    ///
    /// # Arguments
    /// * `service` - The PAM service to use for authentication.
    ///
    /// # Returns
    /// A new `Authenticator` instance.
    pub fn new(service: String) -> Self {
        Self {
            service
        }
    }

    /// Authenticates a user using their credentials.
    ///
    /// # Arguments
    /// * `credentials` - The user's credentials (username and password).
    /// # Returns
    /// A `Result` containing an `AuthenticatedSession` if authentication succeeds,
    /// or a `RouterError` if authentication fails.
    pub fn authenticate(&self, credentials: &Credentials) -> Result<AuthenticatedSession> {
        let environment = self.authenticate_credentials(credentials)?;
        
        if let Ok(Some(user)) = User::from_name(credentials.username()) {
            return match Account::from_user(user) {
                Some(account) => Ok(AuthenticatedSession::new(account, environment)),
                None => Err(RouterError::AuthenticationError(format!("User \"{}\" is invalid. check they have a home directory?", credentials.username())))
            };
        }
        Err(RouterError::AuthenticationError(format!("Could not find user \"{}\"", credentials.username())))
    }

    /// Authenticates a user using their credentials.
    ///
    /// # Arguments
    /// * `credentials` - The user's credentials (username and password).
    ///
    /// # Returns
    /// A `Result` containing an `EnvList` of environment variables if authentication succeeds,
    /// or an `ApplicationError` if authentication fails.
    fn authenticate_credentials(&self, credentials: &Credentials) -> Result<EnvList> {
        // Check for local file authentication of standard username/password
        if credentials.is_credentials_file() {

            credentials.validate_credentials_file()?;

            debug!("Authenticating local user {}", credentials.username());
            self.authenticate_credentials_with_service("su", &credentials)

        } else {
            debug!("Authenticating user {} for service {}", credentials.username(), self.service);
            self.authenticate_credentials_with_service(&self.service, &credentials)
        }
    }

    /// Authenticates a user with a specific PAM service using their credentials.
    /// ///
    /// # Arguments
    /// * `service` - The PAM service to use for authentication.
    /// * `credentials` - The user's credentials (username and password).
    /// ///
    /// # Returns
    /// A `Result` containing an `EnvList` of environment variables if authentication succeeds,
    /// or a `RouterError` if authentication fails.
    fn authenticate_credentials_with_service(&self, service: &str, credentials: &Credentials) -> Result<EnvList> {
        let conversation = Conversation::with_credentials(credentials.username(), credentials.password());
        let mut context = Context::new(service, None, conversation)?;

        context.authenticate(Flag::NONE)?;
        let session = context.open_session(Flag::NONE)?;
        Ok(session.envlist())
    }
}
