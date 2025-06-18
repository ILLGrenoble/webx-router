#[macro_use]
extern crate log;
extern crate dotenv;
extern crate pam_client2 as pam_client;

use crate::app::Application;
use crate::common::{Settings, RouterError, System};

use nix::unistd::{Uid, User};
use structopt::StructOpt;
use dotenv::dotenv;
use std::process;

mod app;
mod authentication;
mod sesman;
mod fs;
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

    if !Uid::effective().is_root() {
        eprintln!("You must run this executable with root permissions");
        std::process::exit(1);
    }

    // Verify we have the webx user
    let webx_user = match System::get_user("webx") {
        Some(user) => user,
        None => {
            error!("The 'webx' user does not exist. Please create it before running the application.");
            process::exit(1);
        }
    };
    
    // Parse command-line arguments.
    let opt = Opt::from_args();

    // Load application settings from the specified configuration file.
    let mut settings = Settings::new(&opt.config).expect("Loaded settings");

    // Initialize logging based on the settings.
    if let Err(error) = setup_logging(&settings) {
        eprintln!("Failed to initialize logging: {}", error);
        process::exit(1);
    }

    if let Err(error) = bootstrap(&settings, &webx_user) {
        eprintln!("Failed to bootstap application: {}", error);
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
            let log_file = std::fs::OpenOptions::new()
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

/// Performs initial setup for the server, including creating necessary directories
/// and ensuring correct permissions.
///
/// # Arguments
/// * `settings` - The configuration settings for the server.
///
/// # Returns
/// A `Result` indicating success or an `ApplicationError`.
fn bootstrap(settings: &Settings, webx_user: &User) -> Result<(), RouterError> {

    fs::mkdir(&settings.sesman.xorg.log_path)?;

    // create the sessions directory
    let sessions_path = &settings.sesman.xorg.sessions_path;
    fs::mkdir(sessions_path)?;
    // ensure permissions and ownership are correct
    fs::chown(sessions_path, webx_user.uid.as_raw(), webx_user.gid.as_raw())?;
    fs::chmod(sessions_path, 0o755)?;
    
    Ok(())
}
