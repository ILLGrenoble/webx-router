pub use event_bus::{EventBus, APPLICATION_SHUTDOWN_COMMAND, INPROC_APP_TOPIC, INPROC_SESSION_TOPIC};
pub use error::{RouterError, Result};
pub use settings::{Settings, TransportSettings, SesManSettings, XorgSettings};
pub use system::System;
pub use process_handle::ProcessHandle;

mod event_bus;
mod error;
mod settings;
mod system;
mod process_handle;

use rand::{
    rng, 
    Rng,
    distr::Alphanumeric,
};

/// Converts a camelCase string to snake_case
pub fn to_snake_case(camel_case: &str) -> String {
    let mut result = String::with_capacity(camel_case.len());
    let mut chars = camel_case.chars().peekable();

    while let Some(current) = chars.next() {
        if current.is_uppercase() {
            if !result.is_empty() {
                result.push('_');
            }
            result.push(current.to_lowercase().next().unwrap());
        } else {
            result.push(current);
        }
    }
    result
}

pub fn random_string(length: usize) -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}
