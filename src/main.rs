use crate::utils::{EventBus, APPLICATION_SHUTDOWN_COMMAND};
use crate::router::ClientConnector;
use std::thread;

#[macro_use]
extern crate log;

use env_logger::Env;

mod utils;
mod router;

fn main() {
    let env = Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info");
    env_logger::init_from_env(env);

    info!("Starting WebX Router...");

    // Create ZMQ context
    let context = zmq::Context::new();

    // Create event bus
    let event_bus_thread = create_message_bus_thread(context.clone());

    // Create CTRL-C shutdown publisher
    create_shutdown_publisher(&context);

    // Create client connector
    let connector = ClientConnector::new(context);

    info!("WebX Router running");
    connector.run();

    // Join event bus thread
    event_bus_thread.join().unwrap();

    info!("WebX Router terminated");
}

fn create_message_bus_thread(context: zmq::Context) -> thread::JoinHandle<()>{
    thread::spawn(move || {
        EventBus::new(context).run();
    })
}

fn create_shutdown_publisher(context: &zmq::Context) {
    let socket = EventBus::create_event_publisher(context).unwrap();
    ctrlc::set_handler(move || {
        info!("Sending shutdown command");
        socket.send(APPLICATION_SHUTDOWN_COMMAND, 0).unwrap();

    }).expect("Error setting Ctrl-C handler");
}