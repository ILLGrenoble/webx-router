use crate::connector::Connector;

#[macro_use]
extern crate log;

use env_logger::Env;

mod connector;
mod publisher;

fn main() {
    let env = Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info");
    env_logger::init_from_env(env);

    info!("Starting WebX Router...");

    let connector = Connector::new();
    connector.init();

    // ctrlc::set_handler(|connector| {
    //     connector.stop();
    //
    // }).expect("Error setting Ctrl-C handler");

    let socket_timeout_ms = -1;

    info!("WebX Router running");
    connector.run(socket_timeout_ms);
}
