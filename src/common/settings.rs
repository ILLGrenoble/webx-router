use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct PortSettings {
    pub connector: u32,
    pub publisher: u32,
    pub collector: u32,
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
}

#[derive(Debug, Deserialize, Clone)]
pub struct TransportSettings {
    pub ports: PortSettings,
    pub ipc: IPCSettings,
    pub encryption: EncryptionSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub logging: String,
    pub transport: TransportSettings,
}

impl Settings {
    pub fn new() -> Result<Self, config::ConfigError> {

        let mut settings_raw = config::Config::default();

        settings_raw.merge(config::File::new("config.yml", config::FileFormat::Yaml))?;
        settings_raw.merge(config::Environment::with_prefix("WEBX_ROUTER").separator("_"))?;

        settings_raw.try_into()
    }
}
