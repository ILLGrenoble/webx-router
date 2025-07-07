use crate::common::{Result, RouterError};

pub enum EngineStatus {
    Starting,
    Ready,
}

impl EngineStatus {
    pub fn to_u32(&self) -> u32 {
        match self {
            EngineStatus::Starting => 0,
            EngineStatus::Ready => 1,
        }
    }

    pub fn try_from(value: u32) -> Result<Self> {
        match value {
            0 => Ok(EngineStatus::Starting),
            1 => Ok(EngineStatus::Ready),
            _ => Err(RouterError::SystemError(format!("Failed to convert EngineStatus {}", value))),
        }
    }
}

pub struct EngineSessionInfo {
    secret: String,
    status: EngineStatus,
}

impl EngineSessionInfo {
    pub fn new(secret: String, status: EngineStatus) -> Self {
        Self {
            secret,
            status,
        }
    }

    pub fn secret(&self) -> &str {
        &self.secret
    }

    pub fn status(&self) -> &EngineStatus {
        &self.status
    }
}