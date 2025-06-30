#[macro_use]
extern crate log;

use webx_router::{
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
    /// Port to connect to
    #[structopt(short, long, default_value = "5555")]
    port: u32,

    /// Produces verbose logging
    #[structopt(short, long)]
    verbose: bool,

    #[structopt(subcommand)]
    command: Command,
}

/// Entry point of the WebX CLI application.
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
    
    match opt.command {
        Command::Create {daemon, width, height, keyboard_layout} => {
            match cli.create(width, height, &keyboard_layout) {
                Ok(response) => {
                    match response.code {
                        SessionCreationReturnCodes::Success => {
                            let session_id = response.message;
                            info!("WebX Engine running with session Id {}", &session_id);
                            if !daemon {
                                if let Err(error) = cli.wait_for_interrupt(&session_id) {
                                    error!("Failed to wait for WebX Engine process: {}", error);
                                }
                            }
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

fn setup_logging(verbose: bool) -> Result<(), fern::InitError> {
    let logging_level = if verbose { log::LevelFilter::Debug } else { log::LevelFilter::Info };

    let base_config = fern::Dispatch::new()
        .format(move |out, message, _| {
            out.finish(format_args!("{}", &message.to_string()))
        })
        .level(logging_level)
        .chain(std::io::stdout());

    // Apply the logging configuration.
    base_config.apply()?;
    Ok(())
}

