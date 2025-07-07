use crate::common::{Result, RouterError};

/// Represents the status of a WebX Engine session.
#[derive(PartialEq, Eq)] // Allows equality comparisons
pub enum EngineStatus {
    /// The engine is starting up (waiting for the Xorg process)
    Starting,
    /// The engine is ready for use.
    Ready,
}

impl EngineStatus {
    /// Converts the EngineStatus to a u32 value.
    ///
    /// # Returns
    /// * `0` for Starting
    /// * `1` for Ready
    pub fn to_u32(&self) -> u32 {
        match self {
            EngineStatus::Starting => 0,
            EngineStatus::Ready => 1,
        }
    }

    /// Attempts to create an EngineStatus from a u32 value.
    ///
    /// # Arguments
    /// * `value` - The u32 value to convert.
    ///
    /// # Returns
    /// * `Ok(EngineStatus)` if the value is valid.
    /// * `Err(RouterError)` if the value is invalid.
    pub fn try_from(value: u32) -> Result<Self> {
        match value {
            0 => Ok(EngineStatus::Starting),
            1 => Ok(EngineStatus::Ready),
            _ => Err(RouterError::SystemError(format!("Failed to convert EngineStatus {}", value))),
        }
    }
}

/// Contains information about an engine session, including its secret and status.
pub struct EngineSessionInfo {
    secret: String,
    status: EngineStatus,
}

impl EngineSessionInfo {
    /// Creates a new EngineSessionInfo.
    ///
    /// # Arguments
    /// * `secret` - The session secret.
    /// * `status` - The status of the engine session.
    pub fn new(secret: String, status: EngineStatus) -> Self {
        Self {
            secret,
            status,
        }
    }

    /// Returns a reference to the session secret.
    pub fn secret(&self) -> &str {
        &self.secret
    }

    /// Returns a reference to the engine session status.
    pub fn status(&self) -> &EngineStatus {
        &self.status
    }
}