use crate::common::{Result, RouterError};
use std::process::{Command};

pub struct User {
}

impl User {
    pub fn get_current_username() -> Result<String> {
        match Command::new("whoami")
            .output() {
                Err(error) => Err(RouterError::SystemError(format!("Failed to current username: {}", error))),
                Ok(output) => {
                    // Get stdout
                    let mut stdout = String::from_utf8(output.stdout).expect("Failed to get stdout");

                    // Remove trailing endline
                    let len = stdout.trim_end_matches(&['\r', '\n'][..]).len();
                    stdout.truncate(len);

                    Ok(stdout)
                }
            }
    }

    pub fn change_file_permissions(path: &str, permissions: &str) -> Result<()> {
        match Command::new("chmod")
        .arg(permissions)
        .arg(path)
            .output() {
                Err(error) => Err(RouterError::SystemError(format!("Failed to change file permissions: {}", error))),
                Ok(_) => {
                    debug!("Changed permission of {}", path);
                    Ok(())
                }
            }
    }
}