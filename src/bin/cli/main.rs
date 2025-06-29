#[macro_use]
extern crate log;

use webx_router::{
    common::{Settings},
    app::Cli,
    router::SessionCreationReturnCodes,
};

use structopt::StructOpt;
use dotenv::dotenv;
use std::process;

/// The `Command` enum represents the various commands that the WebX CLI can execute.
#[derive(StructOpt, Debug)]
#[structopt(name = "webx-cli-commands")]
enum Command {
    /// Creates a WebX Session for the current user
    Create {
        /// Specified whether to run in daemon mode or not
        #[structopt(short, long)]
        daemon: bool,

        /// Specified the width of the screen
        #[structopt(short, long)]
        width: u32,

        /// Specified the height of the screen
        #[structopt(short, long)]
        height: u32,

        /// Specified the keyboard layout
        #[structopt(short, long, default_value = "gb")]
        keyboard_layout: String,

    },
    /// List all sessions
    List,
}

/// Command-line options for the WebX CLI.
#[derive(StructOpt, Debug)]
#[structopt(name = "webx-cli-options")]
struct Opt {
    /// Path to the configuration file.
    #[structopt(short, long, default_value = "")]
    config: String,

    #[structopt(subcommand)]
    command: Command,
}

/// Entry point of the WebX CLI application.
fn main() {
    dotenv().ok();

    // Parse command-line arguments.
    let opt = Opt::from_args();

    // Load application settings from the specified configuration file.
    let settings = Settings::new(&opt.config).expect("Loaded settings");

    // Initialize logging based on the settings.
    if let Err(error) = setup_logging(&settings) {
        error!("Failed to initialize logging: {}", error);
        process::exit(1);
    }

    let mut cli = Cli::new();
    if let Err(error) = cli.connect(&settings) {
        error!("Failed to connect: {}", error);
        std::process::exit(1);
    }

    match opt.command {
        Command::Create {daemon, width, height, keyboard_layout} => {
            info!("Got create command with daemon = {}", daemon);
            match cli.create(width, height, &keyboard_layout) {
                Ok(response) => {
                    match response.code {
                        SessionCreationReturnCodes::Success => {
                            info!("WebX Engine running with session Id {}", response.message);
                        },
                        SessionCreationReturnCodes::InvalidRequestParameters => {
                            error!("InvalidRequestParameters: {}", response.message);
                        },
                        SessionCreationReturnCodes::CreationError => {
                            error!("CreationError: {}", response.message);
                        },
                        SessionCreationReturnCodes::AuthenticationError => {
                            error!("InvalidRequestPaAuthenticationErrorrameters: {}", response.message);
                        },
                    }
                },
                Err(error) => error!("Create command failed: {}", error)
            }

        }
        Command::List => {
            info!("Got list command");
        }
    }

    cli.disconnect();

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

    // Enable console logging if configured.
    if logging_config.console.unwrap_or(true) {
        base_config = base_config.chain(std::io::stdout());
    }

    // Apply the logging configuration.
    base_config.apply()?;
    Ok(())
}

