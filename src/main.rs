#[macro_use]
extern crate log;
extern crate dotenv;

use crate::app::Application;
use crate::common::Settings;

use structopt::StructOpt;
use dotenv::dotenv;
use std::process;

use std::fs;

mod app;
mod common;
mod service;
mod router;

/// Command-line options for the WebX Router application.
#[derive(StructOpt, Debug)]
#[structopt(name = "webx-router")]
struct Opt {
    /// Path to the configuration file.
    #[structopt(short, long, default_value = "")]
    config: String,
}

/// Entry point of the WebX Router application.
fn main() {
    dotenv().ok();

    // Parse command-line arguments.
    let opt = Opt::from_args();

    // Load application settings from the specified configuration file.
    let mut settings = Settings::new(&opt.config).expect("Loaded settings");

    // Initialize logging based on the settings.
    if let Err(e) = setup_logging(&settings) {
        eprintln!("Failed to initialize logging: {}", e);
        process::exit(1);
    }

    // Verify the validity of the settings.
    if !settings.verify() {
        error!("Settings are not valid");
        process::exit(1);
    }

    // Start the application.
    if let Err(error) = Application::new().run(&mut settings) {
        error!("{}", error);
        process::exit(1);
    }
}

/// Configures and initializes logging for the application.
///
/// # Arguments
/// * `settings` - Reference to the application settings containing logging configuration.
///
/// # Returns
/// * `Result<(), fern::InitError>` - Indicates success or failure of the logging setup.
fn setup_logging(settings: &Settings) -> Result<(), fern::InitError> {
    let logging_config = &settings.logging;

    let format_string = logging_config.format.clone();
    let mut base_config = fern::Dispatch::new()
        .format(move |out, message, record| {
            let format = format_string
                .as_deref()
                .unwrap_or("[{timestamp}][{level}] {message}");
            let formatted_message = format
                .replace("{timestamp}", &chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string())
                .replace("{level}", &record.level().to_string())
                .replace("{message}", &message.to_string());
            out.finish(format_args!("{}", formatted_message))
        })
        .level(logging_config.level.parse::<log::LevelFilter>().unwrap_or(log::LevelFilter::Info));

    // Enable console logging if configured.
    if logging_config.console.unwrap_or(true) {
        base_config = base_config.chain(std::io::stdout());
    }

    // Enable file logging if configured.
    if let Some(file_config) = &logging_config.file {
        if file_config.enabled.unwrap_or(false) {
            let log_file = fs::OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(&file_config.path)?;
            base_config = base_config.chain(log_file);
        }
    }

    // Apply the logging configuration.
    base_config.apply()?;
    Ok(())
}
