use crate::{
    common::{RouterError, Result, Settings, to_snake_case, System},
    sesman::{X11Session}
};

use super::{Engine, EngineConnector};

use std::process::{Command, Stdio};
use std::os::unix::{
    io::{FromRawFd, IntoRawFd},
    prelude::CommandExt,
};
use std::collections::HashMap;
use std::fs::OpenOptions;

pub struct EngineService {
}

impl EngineService {
    /// Creates a new `SessionService` instance.
    pub fn new() -> Self {
        Self {
        }
    }

    /// Sends a request to a WebX Engine and retrieves the response.
    ///
    /// # Arguments
    /// * `session_id` - The ID of the session.
    /// * `context` - The ZeroMQ context.
    /// * `request` - The request string to send.
    ///
    /// # Returns
    /// The response from the session.
    pub fn send_engine_request(&mut self, engine: &Engine, context: &zmq::Context, request: &str) -> Result<String> {
        let engine_connector = EngineConnector::new(context.clone());
        engine_connector.send_request(&engine.ipc(), request)
    }

    /// Spawns a new WebX Engine process for a session.
    ///
    /// # Arguments
    /// * `x11_session` - The X11 session details.
    /// * `settings` - The application settings.
    /// * `keyboard` - The keyboard layout.
    ///
    /// # Returns
    /// The spawned WebX Engine instance.
    pub fn spawn_engine(&self, x11_session: &X11Session, settings: &Settings, keyboard: &str, engine_parameters: &HashMap<String, String>) -> Result<Engine> {
        let engine_path = &settings.engine.path;
        let engine_log_path = &settings.engine.log_path;
        let message_proxy_path = &settings.transport.ipc.message_proxy;
        let instruction_proxy_path = &settings.transport.ipc.instruction_proxy;
        let engine_connector_root_path = &settings.transport.ipc.engine_connector_root;

        // Get engine log path
        let log_path = format!("{}/webx-engine.{}.log", engine_log_path, x11_session.id());

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;

        let file_descriptor = file.into_raw_fd();
        let file_out = unsafe { Stdio::from_raw_fd(file_descriptor) };

        // Get engine connector IPC path
        let session_connector_path = format!("{}.{}.ipc", engine_connector_root_path, x11_session.id());

        let webx_user = System::get_user("webx")
            .ok_or_else(|| RouterError::EngineSessionError("Failed to retrieve 'webx' user".to_string()))?;

        let mut command = Command::new(engine_path);
        command
            .arg("-k")
            .arg(keyboard)
            .stdout(file_out)
            .env("DISPLAY", x11_session.display_id())
            .env("XAUTHORITY", x11_session.xauthority_file_path())
            .env("WEBX_ENGINE_LOG_LEVEL", "debug")
            .envs(self.convert_engine_parameters(engine_parameters))
            .env("WEBX_ENGINE_IPC_SESSION_CONNECTOR_PATH", &session_connector_path)
            .env("WEBX_ENGINE_IPC_MESSAGE_PROXY_PATH", message_proxy_path)
            .env("WEBX_ENGINE_IPC_INSTRUCTION_PROXY_PATH", instruction_proxy_path)
            .env("WEBX_ENGINE_SESSION_ID", x11_session.id())
            .uid(webx_user.uid.as_raw())
            .gid(webx_user.gid.as_raw());

        debug!("Launching WebX Engine \"{}\" on display {}", engine_path, x11_session.display_id());

        debug!("Spawning command: {}", format!("{:?}", command).replace("\"", ""));

        match command.spawn() {
            Err(error) => Err(RouterError::EngineSessionError(format!("Failed to spawn WebX Engine: {}", error))),
            Ok(child) => Ok(Engine::new(child, session_connector_path))
        }
    }

    /// Validates that a WebX Engine is running and responsive.
    ///
    /// # Arguments
    /// * `engine` - The WebX Engine instance.
    /// * `context` - The ZeroMQ context.
    /// * `tries` - The number of validation attempts.
    ///
    /// # Returns
    /// A result indicating success or failure.
    pub fn validate_engine(&self, engine: &Engine, context: &zmq::Context, mut tries: i32) -> Result<()> {
        // Verify session is running
        let engine_connector = EngineConnector::new(context.clone());
        let mut connection_error = "".to_string();
        while tries > 0 {
            match engine_connector.send_request(&engine.ipc(), "ping") {
                Ok(message) => {
                    if message != "pong" {
                        error!("Received non-pong response from WebX Engine on {}: {}", &engine.ipc(), message);
                        return Err(RouterError::EngineSessionError("Received non-pong message".to_string()));
                    }
            
                    trace!("Received pong response from WebX Engine on {}", &engine.ipc());
                    return Ok(());
                },
                Err(error) => {
                    connection_error = error.to_string();
                    tries -= 1;
                }
            }
        }
        Err(RouterError::EngineSessionError(connection_error))
    }

    /// Converts engine parameters into environment variables.
    /// Keys are converted from camelCase to SNAKE_CASE and prefixed with "WEBX_ENGINE_".
    ///
    /// # Arguments
    /// * `parameters` - HashMap containing the engine parameters
    ///
    /// # Returns
    /// A vector of tuples containing the environment variable name and value
    fn convert_engine_parameters(&self, parameters: &HashMap<String, String>) -> Vec<(String, String)> {
        parameters
            .iter()
            .map(|(key, value)| {
                let snake_case = to_snake_case(key);
                let env_key = format!("WEBX_ENGINE_{}", snake_case.to_uppercase());
                (env_key, value.clone())
            })
            .collect()
    }

}