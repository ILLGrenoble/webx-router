use serde::Deserialize;
use std::fs;
use std::path::Path;

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
    pub sesman_connector: String,
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
    pub auto_logout_s: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FileLoggingSettings {
    pub enabled: Option<bool>,
    pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingSettings {
    pub level: String,
    pub console: Option<bool>,
    pub file: Option<FileLoggingSettings>,
    pub format: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub logging: LoggingSettings,
    pub transport: TransportSettings,
    pub sesman: SesManSettings,
    pub engine: EngineSettings,
}

static DEFAULT_CONFIG_PATHS: [&str; 2] = ["/etc/webx/webx-router-config.yml", "./config.yml"];

impl Settings {
    pub fn new(config_path: &str) -> Result<Self, config::ConfigError> {

        let config_path = Settings::get_config_path(config_path);

        let settings_raw = config::Config::builder()
            .add_source(config::File::new(config_path, config::FileFormat::Yaml))
            .add_source(config::Environment::with_prefix("WEBX_ROUTER").separator("_"))
            .build()?;        
 
        settings_raw.try_deserialize()
   }

    pub fn verify(&self) -> bool {
        // Check that settings are valid for running a router

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

    fn get_config_path(config_path: &str) -> &str {
        if config_path == "" {
            for path in DEFAULT_CONFIG_PATHS.iter() {
                if Path::new(path).exists() {
                    return path;
                }
            }
        }
        return config_path;
    }
}
