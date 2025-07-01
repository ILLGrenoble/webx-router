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


/// The `IPCSettings` struct represents the inter-process communication configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct IPCSettings {
    /// The path to the message proxy.
    pub message_proxy: String,
    /// The path to the instruction proxy.
    pub instruction_proxy: String,
    /// The root path for engine connectors.
    pub engine_connector_root: String,
}

/// The `TransportSettings` struct represents the transport configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct TransportSettings {
    /// The port settings for various services.
    pub ports: PortSettings,
    /// The IPC settings for inter-process communication.
    pub ipc: IPCSettings,
}

/// The `EngineSettings` struct represents the WebX Engine configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct EngineSettings {
    /// The path to the WebX Engine binary.
    pub path: String,
    /// The directory for storing engine logs.
    pub log_path: String,
}

/// The `XorgSettings` struct contains settings related to the Xorg server.
#[derive(Debug, Deserialize, Clone)]
pub struct XorgSettings {
    pub log_path: String,
    pub lock_path: String,
    pub sessions_path: String,
    pub config_path: String,
    pub display_offset: u32,
    pub window_manager: String,
}

/// The `AuthenticationSettings` struct contains settings for user authentication.
#[derive(Debug, Deserialize, Clone)]
pub struct AuthenticationSettings {
    pub service: String,
}

/// The `SesManSettings` struct represents the session manager configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct SesManSettings {
    pub authentication: AuthenticationSettings,
    pub xorg: XorgSettings,
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

impl XorgSettings {
    pub fn sessions_path_for_uid(&self, uid: u32) -> String {
        format!("{}/{}", self.sessions_path, uid)
    }
}

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


        if self.logging.level.is_empty() {
            eprintln!("Please specify a logging level (trace, debug, info, error)");
            return false;
        }

        if self.logging.file.is_some() {
            let file = self.logging.file.as_ref().unwrap();

            if file.enabled.unwrap() && file.path.is_empty() {
                eprintln!("Please specify a path for the log file");
                return false;
            }
        }

        if self.sesman.authentication.service.is_empty() {
            eprintln!("Please specify a PAM service to use (i.e. login)");
            return false;
        }

        if self.sesman.xorg.sessions_path.is_empty() {
            eprintln!("Please specify a path for where to store the session files (i.e. /run/webx/sessions");
            return false;
        }

        if self.sesman.xorg.lock_path.is_empty() {
            eprintln!("Please specify a path for where to look for x lock files (i.e. /tmp/.X11-unix");
            return false;
        }

        if self.sesman.xorg.window_manager.is_empty() {
            eprintln!("Please specify a path to a command that will launch your chosen session manager");
            return false;
        }

        if self.sesman.xorg.log_path.is_empty() {
            eprintln!("Please specify a path to store the session logs i.e. /var/log/webx/webx-session-manager/sessions");
            return false;
        }

        // Verify engine path is set
        if self.engine.path.is_empty() {
            error!("Engine path is missing from settings");
            return false;
        }

        // Verify engine log dir
        if let Err(error) = fs::create_dir_all(&self.engine.log_path) {
            error!("Cannot create engine log directory at {}: {}", self.engine.log_path, error);
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
