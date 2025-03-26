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

#[derive(StructOpt, Debug)]
#[structopt(name = "webx-router")]
struct Opt {
    /// Config path
    #[structopt(short, long, default_value = "")]
    config: String,
}

fn main() {
    dotenv().ok();
    let opt = Opt::from_args();

    let mut settings = Settings::new(&opt.config).expect("Loaded settings");

    // Initialize logging
    if let Err(e) = setup_logging(&settings) {
        eprintln!("Failed to initialize logging: {}", e);
        process::exit(1);
    }

    // Verify settings
    if !settings.verify() {
        error!("Settings are not valid");
        process::exit(1);
    }

    if let Err(error) = Application::new().run(&mut settings) {
        error!("{}", error);
        process::exit(1);
    }

}

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

    if logging_config.console.unwrap_or(true) {
        base_config = base_config.chain(std::io::stdout());
    }

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

    base_config.apply()?;
    Ok(())
}
