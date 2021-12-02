use crate::message_bus::{MessageBus, APPLICATION_SHUTDOWN_COMMAND};
use crate::connector::Connector;
use std::thread;

#[macro_use]
extern crate log;

use env_logger::Env;

mod common;
mod message_bus;
mod connector;
mod pub_sub_proxy;
mod pull_push_proxy;

fn main() {
    let env = Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info");
    env_logger::init_from_env(env);

    info!("Starting WebX Router...");

    // Create ZMQ context
    let context = zmq::Context::new();

    // Create message bus
    let message_bus_thread = create_message_bus_thread(context.clone());

    // Create CTRL-C shutdown publisher
    create_shutdown_publisher(&context);

    // Create client connector
    let connector = Connector::new(context);

    info!("WebX Router running");
    connector.run();

    // Join message-bus thread
    message_bus_thread.join().unwrap();

    info!("WebX Router terminated");
}

fn create_message_bus_thread(context: zmq::Context) -> thread::JoinHandle<()>{
    thread::spawn(move || {
        MessageBus::new(context).run();
    })
}

fn create_shutdown_publisher(context: &zmq::Context) {
    let socket = MessageBus::create_message_publisher(context).unwrap();
    ctrlc::set_handler(move || {
        info!("Sending shutdown command");
        socket.send(APPLICATION_SHUTDOWN_COMMAND, 0).unwrap();

    }).expect("Error setting Ctrl-C handler");
}