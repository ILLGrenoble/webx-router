use std::ffi::CString;
use std::fs;
use std::fs::{OpenOptions, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::common::{Result, RouterError};

// Group and other read/write bits
const GROUP_READ: u32 = 0o040;
const GROUP_WRITE: u32 = 0o020;
const OTHER_READ: u32 = 0o004;
const OTHER_WRITE: u32 = 0o002;

/// Changes the ownership of a file or directory.
///
/// # Arguments
/// * `path` - The path to the file or directory.
/// * `uid` - The user ID to set as the owner.
/// * `gid` - The group ID to set as the owner.
///
/// # Returns
/// A `Result` indicating success or a `RouterError` if the operation fails.
pub fn chown(path: &str, uid: u32, gid: u32) -> Result<()> {
    let cpath =
        CString::new(path).map_err(|error| RouterError::SystemError(format!("{}", error)))?;
    match unsafe { libc::chown(cpath.as_ptr(), uid, gid) } {
        0 => Ok(()),
        code => Err(RouterError::SystemError(format!("Error changing ownership of file {}: {}", path, code))),
    }
}

/// Creates a directory and all its parent directories if they do not exist.
///
/// # Arguments
/// * `path` - The path to the directory to create.
///
/// # Returns
/// A `Result` indicating success or a `RouterError` if the operation fails.
pub fn mkdir(path: &str) -> Result<()> {
    if fs::create_dir_all(path).is_err() {
        return Err(RouterError::SystemError(format!("Could create directory for path: {}", path)));
    }
    Ok(())
}

/// Changes the permissions of a file or directory.
///
/// # Arguments
/// * `path` - The path to the file or directory.
/// * `mode` - The permissions to set, in octal format (e.g., `0o755`).
///
/// # Returns
/// A `Result` indicating success or a `RouterError` if the operation fails.
pub fn chmod(path: &str, mode: u32) -> Result<()> {
    let mode = Permissions::from_mode(mode);
    if fs::set_permissions(path, mode).is_err() {
        return Err(RouterError::SystemError(format!("Could not change permissions: {}", path)));
    }
    Ok(())
}

/// Creates a new file or updates the last modified time if the file already exists.
///
/// # Arguments
/// * `path` - The path to the file to create or update.
///
/// # Returns
/// A `Result` indicating success or a `RouterError` if the operation fails.
pub fn touch(path: &str) -> Result<()> {
    if OpenOptions::new()
        .create_new(true)
        .write(true)
        .append(true)
        .open(path)
        .is_err()
    {
        return Err(RouterError::SystemError(format!("Could not create file: {}", path)));
    }
    Ok(())
}

/// Checks if a file exists at the specified path.
///
/// # Arguments
/// * `path` - The path to the file to check.
///
/// # Returns
/// * `bool` - `true` if the file exists, `false` otherwise.
pub fn file_exists(path: &str) -> bool {
    Path::new(path).exists()
}

/// Retrieves the metadata for a file at the specified path, if it exists.
///
/// # Arguments
/// * `path` - The path to the file.
///
/// # Returns
/// * `Option<fs::Metadata>` - Some(metadata) if the file exists and metadata can be retrieved, None otherwise.
pub fn file_params(path: &str) -> Option<fs::Metadata> {
    if file_exists(path) {
        match fs::metadata(Path::new(path)) {
            Ok(metadata) => Some(metadata),
            Err(error) => {
                warn!("Unable obtain metadata from file at {}: {}", path, error);
                None
            }
        }
    } else {
        warn!("Unable obtain metadata from file at {}: File doesn't exist", path);
        None
    }
}

/// Checks if the given mode grants permissions only to the user (no group or other permissions).
///
/// # Arguments
/// * `mode` - The file mode (permission bits) to check.
///
/// # Returns
/// * `bool` - `true` if only the user has permissions, `false` otherwise.
pub fn user_only_permissions(mode: u32) -> bool {
    (mode & (GROUP_READ | GROUP_WRITE | OTHER_READ | OTHER_WRITE)) == 0
}