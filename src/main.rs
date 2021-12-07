use crate::app::Application;
use crate::common::Settings;

#[macro_use]
extern crate log;

use env_logger::Env;

mod app;
mod common;
mod router;

fn main() {
    let settings = Settings::new().expect("Loaded settings");

    let env = Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, &settings.logging);
    env_logger::init_from_env(env);

    if let Err(error) = Application::new().run(settings) {
        error!("{}", error);
    }
}
