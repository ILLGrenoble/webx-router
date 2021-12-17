use crate::common::User;

use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct PortSettings {
    pub connector: u32,
    pub publisher: u32,
    pub collector: u32,
    pub session: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EncryptionSettings {
    pub public: String,
    pub private: String
}

#[derive(Debug, Deserialize, Clone)]
pub struct IPCSettings {
    pub message_proxy: String,
    pub instruction_proxy: String,
    pub engine_connector_root: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TransportSettings {
    pub ports: PortSettings,
    pub ipc: IPCSettings,
    pub encryption: EncryptionSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EngineSettings {
    pub path: String,
    pub logdir: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SesManSettings {
    pub enabled: bool,
    // pub url: String,
    pub fallback_display_id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub logging: String,
    pub transport: TransportSettings,
    pub sesman: SesManSettings,
    pub engine: EngineSettings
}

impl Settings {
    pub fn new() -> Result<Self, config::ConfigError> {

        let mut settings_raw = config::Config::default();

        settings_raw.merge(config::File::new("config.yml", config::FileFormat::Yaml))?;
        settings_raw.merge(config::Environment::with_prefix("WEBX_ROUTER").separator("_"))?;

        settings_raw.try_into()
    }

    pub fn verify(&self) -> bool {
        // Check that settings are valid for running a router

        // Verify we are running as root if sesman is used (production usage)
        let uid = User::get_current_user_uid();
        if uid != 0 {
            if self.sesman.enabled {
                error!("App has to be run as root");
                return false;
            
            } else {
                debug!("App running as non-root user {}", uid);
            }
        }

        // Verify engine path is set
        if self.engine.path.is_empty() {
            error!("Engine path is missing from settings");
            return false;
        }

        // Verify engine log dir
        if let Err(error) = fs::create_dir_all(&self.engine.logdir) {
            error!("Cannot create engine log directory at {}: {}", self.engine.logdir, error);
            return false;
        }

        true
    }
}
