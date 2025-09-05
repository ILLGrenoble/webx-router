use std::fs::{self, File};
use std::os::unix::prelude::CommandExt;
use std::process::Command;

use nix::unistd::{User, Gid, Uid, setgroups, setgid, setuid};
use rand::Rng;
use uuid::Uuid;

use crate::authentication::{AuthenticatedSession, Account};
use crate::common::{Result, RouterError, XorgSettings, ProcessHandle};
use crate::fs::{chmod, chown, mkdir, touch};
use super::{ScreenResolution, X11Session};

/// The `XorgService` struct provides functionality for managing Xorg sessions,
/// including creating, cleaning up, and launching Xorg servers and window managers.
pub struct XorgService {
    settings: XorgSettings,
}

impl XorgService {
    /// Creates a new `XorgService` instance.
    ///
    /// # Arguments
    /// * `settings` - The Xorg settings to use for managing sessions.
    ///
    /// # Returns
    /// A new `XorgService` instance.
    pub fn new(settings: XorgSettings) -> Self {
        Self { 
            settings
        }
    }

    /// Creates an Xauth token, launches the Xorg server, and starts the window manager for a session.
    ///
    /// # Arguments
    /// * `authenticated_session` - The authenticated session containing user account information.
    /// * `resolution` - The screen resolution for the session.
    ///
    /// # Returns
    /// A `Result` containing the created `Session` or an `RouterError`.
    pub fn create_xorg(&self, authenticated_session: &AuthenticatedSession, resolution: ScreenResolution) -> Result<X11Session> {
        let webx_user = User::from_name("webx").unwrap().unwrap();

        let display_number = self.get_next_display()?;
        let authority_file_path = format!("{}/{}/Xauthority", self.settings.sessions_path, authenticated_session.account().uid());

        // create the necessary configuration files
        if let Err(error) = self.create_user_files(authenticated_session.account(), &webx_user, &authority_file_path) {
            return Err(RouterError::X11SessionError(format!("Error occurred setting up the configuration for a session {}", error)));
        }

        self.create_token(display_number, authenticated_session.account(), &authority_file_path, &webx_user)?;

        let display_id = format!(":{}", display_number);
        let session_id = Uuid::new_v4().simple().to_string();

        // spawn the x server
        let xorg = self.spawn_x_server(&session_id, &display_id, &authority_file_path, &resolution, authenticated_session)?;
        
        let session = X11Session::new(
            session_id,
            authenticated_session.clone(),
            display_id,
            authority_file_path.clone(),
            xorg,
            resolution,
        );

        Ok(session)
    }

    pub fn create_window_manager(&self, x11_session: &X11Session) -> Result<ProcessHandle> {
        // Verify that the window manager hasn't already been spawned
        if let Some(window_manager) = x11_session.window_manager(){
            return Ok(window_manager.clone());
        }

        // spawn the window manager
        self.spawn_window_manager(x11_session.id(), x11_session.display_id(), x11_session.xauthority_file_path(), x11_session.authenticated_session())
    }

    /// Generates a random Xauth cookie for authentication.
    /// The cookie is a 32-character string consisting of hex values.
    /// # Returns
    /// A string containing the generated cookie.
    fn create_cookie(&self) -> String {
        let characters: &[u8] = b"ABCDEF0123456789";
        let mut rng = rand::rng();
        (0..32)
            .map(|_| {
                let idx = rng.random_range(0..characters.len());
                characters[idx] as char
            })
            .flat_map(|c| c.to_lowercase())
            .collect()
    }

    /// Creates an Xauth token for a specific display and user.
    ///
    /// # Arguments
    /// * `display` - The display number.
    /// * `account` - The user account for which the token is created.
    /// * `webx_user` - The WebX system user.
    ///
    /// # Returns
    /// A `Result` containing the path to the Xauthority file or an `RouterError
    fn create_token(&self, display: u32, account: &Account, authority_file_path: &str, webx_user: &User) -> Result<()> {
        debug!("Creating xauth token for display {} and user {}", display, account.username());
        let cookie = self.create_cookie();
        let display = format!(":{}", display);
        Command::new("xauth")
            .arg("-f")
            .arg(&authority_file_path)
            .arg("add")
            .arg(display)
            .arg(".")
            .arg(cookie)
            .uid(account.uid())
            .gid(webx_user.gid.as_raw())
            .output()?;

        chmod(&authority_file_path, 0o640)?;
        Ok(())
    }

    /// Spawns an Xorg server process for a session.
    ///
    /// # Arguments
    /// * `session_id` - The unique identifier for the session.
    /// * `display` - The display number.
    /// * `resolution` - The screen resolution for the session.
    /// * `authenticated_session` - The authenticated session containing user account information.
    ///
    /// # Returns
    /// A `Result` containing the `ProcessHandle` for the Xorg server or an `RouterError`.
    fn spawn_x_server(&self,
                      session_id: &str,
                      display: &str,
                      authority_file_path: &str,
                      resolution: &ScreenResolution,
                      authenticated_session: &AuthenticatedSession) -> Result<ProcessHandle> {
        debug!("Launching x server on display {}", display);
        let account = authenticated_session.account();
        let environment = authenticated_session.environment().clone();
        
        let config = &self.settings.config_path;
        let stdout_file = File::create(&format!("{}/{}.xorg.out.log", self.settings.log_path, session_id))?;
        let stderr_file = File::create(&format!("{}/{}.xorg.err.log", self.settings.log_path, session_id))?;

        let xdg_run_time_dir = format!("{}/{}", self.settings.sessions_path, account.uid());
        let mut command = Command::new("Xorg");

        command
            .args([
                display,
                "-auth",
                authority_file_path,
                "-config",
                &config,
                "-verbose",
            ])
            .env_clear()
            .env("DISPLAY", display)
            .env("XAUTHORITY", authority_file_path)
            .env("HOME", account.home())
            .env("XORG_RUN_AS_USER_OK", "1")
            .env("XDG_RUNTIME_DIR", xdg_run_time_dir)
            .env("XRDP_START_WIDTH", resolution.width().to_string())
            .env("XRDP_START_HEIGHT", resolution.height().to_string())
            .envs(environment)
            .current_dir(account.home())
            .stdout(std::process::Stdio::from(stdout_file))
            .stderr(std::process::Stdio::from(stderr_file));

        // Convert u32 groups to Gid and set supplementary groups
        // let gids: Vec<Gid> = account.groups().iter().map(|&g| Gid::from_raw(g)).collect();
        // let uid = Uid::from_raw(account.uid());
        // let gid = Gid::from_raw(account.gid());


        // unsafe {
        //     // The `pre_exec` function is used to set the user and group IDs before executing the command
        //     // This is necessary to ensure the remote desktop runs with the correct permissions
        //     // and can access the user's home directory and other resources.
        //     // Alternative the the .groups method of Command could be used but this requires the nightly/unstable version of rust
        //     command
        //         .pre_exec(move || {
        //             setgroups(&gids)?;
        //             setgid(gid)?;
        //             setuid(uid)?;
        //             Ok(())
        //         });
        // }

        debug!("Spawning command: {}", format!("{:?}", command).replace('\"', ""));
        ProcessHandle::new(&mut command).map_err(|e| {
            error!("Failed to spawn Xorg server process: {}", e);
            RouterError::X11SessionError(format!("Failed to spawn Xorg server: {}", e))
        })
    }

    /// Spawns a window manager process for a session.
    ///
    /// # Arguments
    /// * `session_id` - The unique identifier for the session.
    /// * `display` - The display id.
    /// * `authority_file_path` - The path to the Xauthority file for the session.
    /// * `authenticated_session` - The authenticated session containing user account information.
    ///
    /// # Returns
    /// A `Result` containing the `ProcessHandle` for the window manager or an `RouterError`.
    fn spawn_window_manager(&self,
                            session_id: &str,
                            display: &str,
                            authority_file_path: &str,
                            authenticated_session: &AuthenticatedSession) -> Result<ProcessHandle> {

        let account = authenticated_session.account();
        let environment = authenticated_session.environment().clone();

        let log_path = &self.settings.log_path;
        let stdout_file = File::create(&format!("{}/{}.wm.out.log", log_path, session_id))?;
        let stderr_file = File::create(&format!("{}/{}.wm.err.log", log_path, session_id))?;

        let xdg_run_time_dir = self.settings.sessions_path_for_uid(account.uid());

        let mut command = Command::new(&self.settings.window_manager);

        command
            .env_clear()
            .env("DISPLAY", display)
            .env("XAUTHORITY", authority_file_path)
            .env("HOME", account.home())
            .env("XDG_RUNTIME_DIR", xdg_run_time_dir)
            .envs(environment)
            .current_dir(account.home())
            .stdout(std::process::Stdio::from(stdout_file))
            .stderr(std::process::Stdio::from(stderr_file));

        // Convert u32 groups to Gid and set supplementary groups
        let gids: Vec<Gid> = account.groups().iter().map(|&g| Gid::from_raw(g)).collect();
        let uid = Uid::from_raw(account.uid());
        let gid = Gid::from_raw(account.gid());

        unsafe {
            // The `pre_exec` function is used to set the user and group IDs before executing the command
            // This is necessary to ensure the remote desktop runs with the correct permissions
            // and can access the user's home directory and other resources.
            // Alternative the the .groups method of Command could be used but this requires the nightly/unstable version of rust
            command
                .pre_exec(move || {
                    setgroups(&gids)?;
                    setgid(gid)?;
                    setuid(uid)?;

                    Ok(())
                });
        }            

        debug!("Spawning command: {}", format!("{:?}", command).replace('\"', ""));
        ProcessHandle::new(&mut command).map_err(|e| {
            error!("Failed to spawn window manager process: {}", e);
            RouterError::X11SessionError(format!("Failed to spawn window manager: {}", e))
        })
    }

    /// Creates a directory for a session with the specified permissions and ownership.
    ///
    /// # Arguments
    /// * `path` - The path to the directory.
    /// * `mode` - The permissions for the directory.
    /// * `uid` - The user ID to set as the owner.
    /// * `gid` - The group ID to set as the owner.
    ///
    /// # Returns
    /// A `Result` indicating success or an `RouterError`.
    fn create_session_directory<S>(&self, path: S, mode: u32, uid: u32, gid: u32) -> Result<()> where S: AsRef<str> {
        let path = path.as_ref();
        mkdir(path)?;
        // ensure permissions and ownership are correct
        chown(path, uid, gid)?;
        chmod(path, mode)?;
        Ok(())
    }

    /// Creates a user-specific file with the specified permissions and ownership.
    ///
    /// # Arguments
    /// * `path` - The path to the file.
    /// * `mode` - The permissions for the file.
    /// * `uid` - The user ID to set as the owner.
    /// * `gid` - The group ID to set as the owner.
    ///
    /// # Returns
    /// A `Result` indicating success or an `RouterError`.
    fn create_user_file<S>(&self, path: S, mode: u32, uid: u32, gid: u32) -> Result<()> where S: AsRef<str> {
        let path = path.as_ref();

        if fs::metadata(path).is_err() {
            touch(path)?;
        }

        // ensure permissions and ownership are correct
        chmod(path, mode)?;
        debug!("Changing ownership of file to {}:{}", uid, gid);
        chown(path, uid, gid)?;
        Ok(())
    }

    /// Creates the required directories and files for a user session.
    ///
    /// # Arguments
    /// * `account` - The user account for the session.
    /// * `webx_user` - The WebX system user.
    ///
    /// # Returns
    /// A `Result` indicating success or an `RouterError`.
    pub fn create_user_files(&self, account: &Account, webx_user: &User, authority_file_path: &str) -> Result<()> {
        debug!("Creating user files for user: {}", account.username());
        let gid = webx_user.gid.as_raw();
        let uid = account.uid();
        self.create_session_directory(format!("{}/{}", self.settings.sessions_path, uid), 0o750, uid, gid)?;
        self.create_user_file(authority_file_path, 0o640, uid, gid)?;
        Ok(())
    }

    /// Finds the next available display number for a session.
    ///
    /// # Arguments
    /// * `id` - The starting display number to check.
    ///
    /// # Returns
    /// A `Result` containing the next available display number or an `RouterError`.
    fn get_next_available_display(&self, id: u32) -> Result<u32> {
        let lock_path = &self.settings.lock_path;
        let path = format!("{}/.X{}-lock", lock_path, id);
        if fs::metadata(path).is_ok() {
            self.get_next_available_display(id + 1)
        } else {
            Ok(id)
        }
    }

    /// Retrieves the next available display number starting from the configured offset.
    ///
    /// # Returns
    /// A `Result` containing the next available display number or an `RouterError`.
    fn get_next_display(&self) -> Result<u32> {
        let display_offset = self.settings.display_offset;
        self.get_next_available_display(display_offset)
    }
}
