pub enum EngineStatus {
    Starting,
    Ready,
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