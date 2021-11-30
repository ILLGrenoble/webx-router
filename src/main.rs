use crate::connector::Connector;
use crate::inproc_communicator::{ProcessCommunicator, SHUTDOWN_COMMAND};

#[macro_use]
extern crate log;

use env_logger::Env;

mod connector;
mod publisher;
mod inproc_communicator;

fn main() {
    let env = Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info");
    env_logger::init_from_env(env);

    info!("Starting WebX Router...");

    let context = zmq::Context::new();
    create_shutdown_publisher(&context);

    let mut connector = Connector::new(context);
    connector.init();

    info!("WebX Router running");
    connector.run();

    info!("WebX Router terminated");
}

fn create_shutdown_publisher(context: &zmq::Context) {
    let socket = ProcessCommunicator::create_inproc_publisher(context).unwrap();
    ctrlc::set_handler(move || {
        info!("Sending shutdown command");
        socket.send(SHUTDOWN_COMMAND, 0).unwrap();

    }).expect("Error setting Ctrl-C handler");
}