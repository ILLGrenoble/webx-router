use std::error::Error;
use std::result;
use std::fmt;

pub type Result<T> = result::Result<T, RouterError>;

#[derive(Debug)]
pub enum RouterError {
    Transport(String),
    IoError(std::io::Error),
}

impl Error for RouterError {}

impl fmt::Display for RouterError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RouterError::Transport(message) =>write!(formatter, "{}", message),
            RouterError::IoError(err) => {
                writeln!(formatter, "IoError: {}", err)
            },
        }
    }
}

impl From<zmq::Error> for RouterError {
    fn from(err: zmq::Error) -> Self {
        RouterError::Transport(err.to_string())
    }
}

impl From<std::io::Error> for RouterError {
    fn from(err: std::io::Error) -> Self {
        RouterError::IoError(err)
    }
}
