use crate::common::{Result, RouterError};
use std::process::{Command};
use std::fs;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct System {
}

impl System {
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

    pub fn chmod(path: &str, mode: u32) -> Result<()> {
        let mode = Permissions::from_mode(mode);
        if fs::set_permissions(path, mode).is_err() {
            return Err(RouterError::SystemError(format!("Could not change permissions: {}", path)));
        }

        debug!("Changed permission of {}", path);
        Ok(())
    }

    pub fn current_time_s() -> u64 {
        if let Ok(current_time) = SystemTime::now().duration_since(UNIX_EPOCH) {
            current_time.as_secs()
     
        } else {
            0
        }
    }
}