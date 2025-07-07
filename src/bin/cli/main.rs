#[macro_use]
extern crate log;

use webx_router::{
    app::Cli,
    router::SessionCreationReturnCodes,
    engine::EngineStatus,
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
        /// Specifies whether to run in daemon mode or not
        #[structopt(short, long)]
        daemon: bool,

        /// Specifies the width of the screen
        #[structopt(short, long)]
        width: u32,

        /// Specifies the height of the screen
        #[structopt(short, long)]
        height: u32,

        /// Specifies the keyboard layout
        #[structopt(short, long, default_value = "gb")]
        keyboard_layout: String,
    },
    /// Lists all sessions
    List,
}

/// Command-line options for the WebX CLI.
#[derive(StructOpt, Debug)]
#[structopt(name = "webx-cli-options")]
struct Opt {
    /// Port to connect to
    #[structopt(short, long, default_value = "5555")]
    port: u32,

    /// Produces verbose logging
    #[structopt(short, long)]
    verbose: bool,

    /// The subcommand to execute
    #[structopt(subcommand)]
    command: Command,
}

/// Entry point of the WebX CLI application.
///
/// Parses command-line arguments, sets up logging, connects to the WebX Router,
/// and executes the specified command.
///
/// # Arguments
/// None
///
/// # Returns
/// Nothing. Exits the process on error.
fn main() {
    dotenv().ok();

    // Parse command-line arguments.
    let opt = Opt::from_args();

    // Initialize logging based on the settings.
    if let Err(error) = setup_logging(opt.verbose) {
        error!("Failed to initialize logging: {}", error);
        process::exit(1);
    }

    let mut cli = Cli::new();
    if let Err(error) = cli.connect(opt.port) {
        error!("Failed to connect: {}", error);
        std::process::exit(1);
    }
    
    let mut exit_code = 0;
    match opt.command {
        Command::Create {daemon, width, height, keyboard_layout} => {
            match cli.create(width, height, &keyboard_layout) {
                Ok(response) => {
                    match response.code {
                        SessionCreationReturnCodes::Success => {
                            let session_id = response.message;
                            match response.engine_status.as_ref().unwrap_or(&EngineStatus::Starting) {
                                EngineStatus::Ready => info!("WebX Engine Session with Id {} has been created and is ready", &session_id),
                                EngineStatus::Starting => info!("WebX Engine Session with Id {} is being created.", &session_id)
                            }
                            info!("WebX Engine running with session Id {}", &session_id);
                            if !daemon {
                                if let Err(error) = cli.wait_for_interrupt(&session_id, response.engine_status.unwrap_or(EngineStatus::Starting)) {
                                    error!("Failed to wait for WebX Engine process: {}", error);
                                    exit_code = 1;
                                }
                            }
                        },
                        SessionCreationReturnCodes::InvalidRequestParameters => {
                            error!("InvalidRequestParameters: {}", response.message);
                            exit_code = 1;
                        },
                        SessionCreationReturnCodes::CreationError => {
                            error!("CreationError: {}", response.message);
                            exit_code = 1;
                        },
                        SessionCreationReturnCodes::AuthenticationError => {
                            error!("AuthenticationError: {}", response.message);
                            exit_code = 1;
                        },
                    }
                },
                Err(error) => {
                    error!("Create command failed: {}", error);
                    exit_code = 1;
                }
            }

        }
        Command::List => {
            match cli.list() {
                Ok(response) => {
                    info!("Current WebX sessions:\n{}", &response);
                },
                Err(error) => error!("List command failed: {}", error)
            }
        }
    }

    cli.disconnect();

    std::process::exit(exit_code);
}

/// Sets up logging for the CLI application.
///
/// # Arguments
/// * `verbose` - If true, sets the logging level to Debug; otherwise, Info.
///
/// # Returns
/// * `Result<(), fern::InitError>` - Ok if logging is set up successfully, Err otherwise.
fn setup_logging(verbose: bool) -> Result<(), fern::InitError> {
    let logging_level = if verbose { log::LevelFilter::Debug } else { log::LevelFilter::Info };

    let base_config = fern::Dispatch::new()
        .format(move |out, message, _| {
            out.finish(format_args!("{}  {}", &chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string(), &message.to_string()))
        })
        .level(logging_level)
        .chain(std::io::stdout());

    // Apply the logging configuration.
    base_config.apply()?;
    Ok(())
}