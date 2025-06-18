pub use event_bus::{EventBus, APPLICATION_SHUTDOWN_COMMAND, INPROC_APP_TOPIC, INPROC_SESSION_TOPIC};
pub use error::{RouterError, Result};
pub use settings::{Settings, TransportSettings, SesManSettings, XorgSettings};
pub use system::System;
pub use engine_session::EngineSession;
pub use engine_session_container::EngineSessionContainer;
pub use engine::Engine;

mod event_bus;
mod error;
mod settings;
mod system;
mod engine_session;
mod engine_session_container;
mod engine;


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
