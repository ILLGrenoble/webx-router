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