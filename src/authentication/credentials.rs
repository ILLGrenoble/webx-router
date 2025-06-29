use crate::common::{Result, RouterError};
use crate::fs::{file_params, user_only_permissions};
use std::os::unix::fs::MetadataExt;
use users::{get_user_by_uid};

/// The `Credentials` struct represents a user's login credentials, including their username and password.
#[derive(Clone)]
pub struct Credentials {
    username: String,
    password: String,
    credentials_file: Option<String>,
}

impl Credentials {
    pub fn new(username: String, password: String) -> Result<Credentials> {
        let credentials_file = username.to_string();
        if credentials_file.starts_with("/") {
            let metadata = file_params(&credentials_file)
                .ok_or_else(|| RouterError::AuthenticationError(format!("Unable to obtain metadata from credentials file {}", &credentials_file)))?;

            // verify file permissions
            let permissions = metadata.mode();
            if !user_only_permissions(permissions) {
                return Err(RouterError::AuthenticationError(format!("Credentials file {} has insecure permissions", &credentials_file)));
            }

            // Get username from uid
            let uid = metadata.uid();
            let user = get_user_by_uid(uid)
                .ok_or_else(|| RouterError::AuthenticationError(format!("Unable to obtain username from credentials file {} with owner uid {}", &credentials_file, uid)))?;
            let username = user.name().to_string_lossy().into_owned();

            Ok(Self { username, password, credentials_file: Some(credentials_file) })

        } else {
            Ok(Self { username, password, credentials_file: None })
        }
    }

    /// Returns the username of the user.
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Returns the password of the user.
    pub fn password(&self) -> &str {
        &self.password
    }

    pub fn is_credentials_file(&self) -> bool {
        return self.credentials_file.is_some();
    }

    pub fn validate_credentials_file(&self) -> Result<()> {
        if let Some(credentials_file) = &self.credentials_file {
            let mut password = match std::fs::read_to_string(&credentials_file) {
                Ok(password) => password,
                Err(error) => {
                    return Err(RouterError::AuthenticationError(format!("Failed to read from credentials file {}: {}", credentials_file, error)));
                }
            };

            if password.ends_with('\n') {
                password.pop();
            }

            if password == self.password {
                Ok(())
            } else {
                Err(RouterError::AuthenticationError(format!("Password from credentials file {} is incorrect", credentials_file)))
            }
        } else {
            Err(RouterError::AuthenticationError(format!("Credentials does not use a credentials file")))
        }
    }
}
