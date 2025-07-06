pub enum EngineStatus {
    Starting,
    Ready,
    Error(String),
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

    pub fn set_status(&mut self, status: EngineStatus) {
        self.status = status;
    }
}