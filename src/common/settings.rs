use serde::Deserialize;
use std::fs;
use std::path::Path;

/// The `PortSettings` struct represents the port configuration for various services.
#[derive(Debug, Deserialize, Clone)]
pub struct PortSettings {
    /// The port for the connector service.
    pub connector: u32,
    /// The port for the publisher service.
    pub publisher: u32,
    /// The port for the collector service.
    pub collector: u32,
    /// The port for the session service.
    pub session: u32,
}

/// The `EncryptionSettings` struct represents the encryption configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct EncryptionSettings {
    /// The public key for encryption.
    pub public: String,
    /// The private key for encryption.
    pub private: String,
}

/// The `IPCSettings` struct represents the inter-process communication configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct IPCSettings {
    /// The path to the message proxy.
    pub message_proxy: String,
    /// The path to the instruction proxy.
    pub instruction_proxy: String,
    /// The root path for engine connectors.
    pub engine_connector_root: String,
    /// The path to the session manager connector.
    pub sesman_connector: String,
}

/// The `TransportSettings` struct represents the transport configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct TransportSettings {
    /// The port settings for various services.
    pub ports: PortSettings,
    /// The IPC settings for inter-process communication.
    pub ipc: IPCSettings,
    /// The encryption settings for secure communication.
    pub encryption: EncryptionSettings,
}

/// The `EngineSettings` struct represents the WebX Engine configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct EngineSettings {
    /// The path to the WebX Engine binary.
    pub path: String,
    /// The directory for storing engine logs.
    pub logdir: String,
}

/// The `SesManSettings` struct represents the session manager configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct SesManSettings {
    /// Indicates whether the session manager is enabled.
    pub enabled: bool,
    /// The fallback display ID.
    pub fallback_display_id: String,
    /// The auto-logout timeout in seconds.
    pub auto_logout_s: u64,
}

/// The `FileLoggingSettings` struct represents the file logging configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct FileLoggingSettings {
    /// Indicates whether file logging is enabled.
    pub enabled: Option<bool>,
    /// The path to the log file.
    pub path: String,
}

/// The `LoggingSettings` struct represents the logging configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct LoggingSettings {
    /// The logging level (e.g., debug, info, error).
    pub level: String,
    /// Indicates whether console logging is enabled.
    pub console: Option<bool>,
    /// The file logging settings.
    pub file: Option<FileLoggingSettings>,
    /// The logging format.
    pub format: Option<String>,
}

/// The `Settings` struct represents the application configuration settings.
#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    /// The logging-related settings.
    pub logging: LoggingSettings,
    /// The transport-related settings.
    pub transport: TransportSettings,
    /// The session manager-related settings.
    pub sesman: SesManSettings,
    /// The WebX Engine-related settings.
    pub engine: EngineSettings,
}

static DEFAULT_CONFIG_PATHS: [&str; 2] = ["/etc/webx/webx-router-config.yml", "./config.yml"];

impl Settings {
    /// Creates a new `Settings` instance by loading the configuration from a file or environment variables.
    ///
    /// # Arguments
    /// * `config_path` - The path to the configuration file.
    ///
    /// # Returns
    /// A `Result` containing the `Settings` instance or a configuration error.
    pub fn new(config_path: &str) -> Result<Self, config::ConfigError> {

        let config_path = Settings::get_config_path(config_path);

        let settings_raw = config::Config::builder()
            .add_source(config::File::new(config_path, config::FileFormat::Yaml))
            .add_source(config::Environment::with_prefix("WEBX_ROUTER").separator("_"))
            .build()?;        
 
        settings_raw.try_deserialize()
   }

    /// Verifies the validity of the settings.
    ///
    /// # Returns
    /// `true` if the settings are valid, `false` otherwise.
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

    /// Retrieves the configuration file path.
    ///
    /// # Arguments
    /// * `config_path` - The provided configuration file path.
    ///
    /// # Returns
    /// The resolved configuration file path.
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
