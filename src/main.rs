#[macro_use]
extern crate log;
extern crate dotenv;

use crate::app::Application;
use crate::common::Settings;

use env_logger::Env;
use dotenv::dotenv;

mod app;
mod common;
mod service;
mod router;

fn main() {
    dotenv().ok();
    
    let mut settings = Settings::new().expect("Loaded settings");

    let env = Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, &settings.logging);
    env_logger::init_from_env(env);

    if let Err(error) = Application::new().run(&mut settings) {
        error!("{}", error);
    }
}
