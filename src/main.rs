use crate::app::Application;

#[macro_use]
extern crate log;

use env_logger::Env;

mod app;
mod common;
mod router;

fn main() {
    let env = Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info");
    env_logger::init_from_env(env);

    if let Err(error) = Application::new().run() {
        error!("{}", error);
    }
}
