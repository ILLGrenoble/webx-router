use crate::common::{Result, RouterError};
use std::process::{Command};

extern "C" {
    pub fn geteuid() -> u32;
}

pub struct User {
}

impl User {
    pub fn get_current_user_uid() -> u32 {
        let uid = unsafe { geteuid() };
        uid
    }

    pub fn get_uid_for_username(username: &str) -> Result<u32> {
        match Command::new("id")
            .arg("-u")
            .arg(username)
            .output() {
                Err(error) => Err(RouterError::SystemError(format!("Failed to get UID from username {}: {}", username, error))),
                Ok(output) => {
                    // convert output to u32
                    let mut stdout = String::from_utf8(output.stdout).expect("Failed to get stdout");

                    // Remove trailing endline
                    let len = stdout.trim_end_matches(&['\r', '\n'][..]).len();
                    stdout.truncate(len);

                    // Convert output to u32
                    match stdout.parse::<u32>() {
                        Err(_) => Err(RouterError::SystemError(format!("Failed to parse UID from username: {}: {}", username, stdout))),
                        Ok(uid) => Ok(uid)
                    }
                }
            }

    }
}