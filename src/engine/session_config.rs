use crate::sesman::ScreenResolution;
use std::collections::HashMap;

/// Represents the configuration for a user session.
/// This includes the keyboard layout, screen resolution, and engine parameters.
pub struct SessionConfig {
    keyboard_layout: String,
    resolution: ScreenResolution,
    engine_parameters: HashMap<String, String>,
}

impl SessionConfig {
    /// Creates a new `SessionConfig` instance.
    ///
    /// # Arguments
    /// * `keyboard_layout` - The keyboard layout for the session.
    /// * `resolution` - The screen resolution for the session.
    /// * `engine_parameters` - Additional parameters for the engine.
    ///
    /// # Returns
    /// * `SessionConfig` - A new instance of the session configuration.
    pub fn new(
        keyboard_layout: String,
        resolution: ScreenResolution,
        engine_parameters: HashMap<String, String>,
    ) -> Self {
        Self {
            keyboard_layout,
            resolution,
            engine_parameters,
        }
    }

    /// Gets the keyboard layout for the session.
    ///
    /// # Returns
    /// * `String` - The keyboard layout.
    pub fn keyboard_layout(&self) -> &String {
        &self.keyboard_layout
    }

    /// Gets the screen resolution for the session.
    ///
    /// # Returns
    /// * `ScreenResolution` - The screen resolution.
    pub fn resolution(&self) -> &ScreenResolution {
        &self.resolution
    }
    
    /// Gets the engine parameters for the session.
    /// ///
    /// # Returns
    /// * `HashMap<String, String>` - The engine parameters.
    pub fn engine_parameters(&self) -> &HashMap<String, String> {
        &self.engine_parameters
    }
}