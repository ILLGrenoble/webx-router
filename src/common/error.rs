use std::error::Error;
use std::num::ParseIntError;
use std::result;
use std::fmt;

pub type Result<T> = result::Result<T, RouterError>;

/// The `RouterError` enum represents various error types that can occur in the application.
#[derive(Debug)]
pub enum RouterError {
    /// Represents a system-level error.
    SystemError(String),
    /// Represents an error related to transport or communication.
    TransportError(String),
    /// Represents an error related to engine sessions.
    EngineSessionError(String),
    /// Represents an error related to x11 sessions.
    X11SessionError(String),
    /// Represents an I/O error.
    IoError(std::io::Error),
    /// Represents a configuration error.
    ConfigError(config::ConfigError),
    /// Represents an authentication error
    AuthenticationError(String),
}

impl Error for RouterError {}

impl fmt::Display for RouterError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RouterError::SystemError(message) => write!(formatter, "SystemError: {}", message),
            RouterError::TransportError(message) => write!(formatter, "TransportError: {}", message),
            RouterError::EngineSessionError(message) => write!(formatter, "EngineSessionError: {}", message),
            RouterError::X11SessionError(message) => write!(formatter, "X11SessionError: {}", message),
            RouterError::IoError(err) => writeln!(formatter, "IoError: {}", err),
            RouterError::ConfigError(err) => writeln!(formatter, "ConfigError: {}", err),
            RouterError::AuthenticationError(message) => writeln!(formatter, "AuthenticationError: {}", message),
        }
    }
}

impl From<zmq::Error> for RouterError {
    fn from(err: zmq::Error) -> Self {
        RouterError::TransportError(err.to_string())
    }
}

impl From<zmq::DecodeError> for RouterError {
    fn from(err: zmq::DecodeError) -> Self {
        RouterError::TransportError(err.to_string())
    }
}

impl From<std::io::Error> for RouterError {
    fn from(err: std::io::Error) -> Self {
        RouterError::IoError(err)
    }
}

impl From<config::ConfigError> for RouterError {
    fn from(err: config::ConfigError) -> Self {
        RouterError::ConfigError(err)
    }
}

impl From<base64::DecodeError> for RouterError {
    fn from(err: base64::DecodeError) -> Self {
        RouterError::SystemError(err.to_string())
    }
}

impl From<std::str::Utf8Error> for RouterError {
    fn from(err: std::str::Utf8Error) -> Self {
        RouterError::SystemError(err.to_string())
    }
}

impl From<serde_json::Error> for RouterError {
    fn from(err: serde_json::Error) -> Self {
        RouterError::SystemError(err.to_string())
    }
}

impl From<ParseIntError> for RouterError {
    fn from(err: ParseIntError) -> Self {
        RouterError::SystemError(err.to_string())
    }
}

impl From<pam_client::Error> for RouterError {
    fn from(error: pam_client::Error) -> Self {
        RouterError::AuthenticationError(format!("{}", error))
    }
}
