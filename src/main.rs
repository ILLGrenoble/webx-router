#[macro_use]
extern crate log;
extern crate dotenv;

use crate::app::Application;
use crate::common::Settings;

use structopt::StructOpt;
use env_logger::Env;
use dotenv::dotenv;
use std::process;

mod app;
mod common;
mod service;
mod router;

#[derive(StructOpt, Debug)]
#[structopt(name = "webx-router")]
struct Opt {
    /// Config path
    #[structopt(short, long, default_value = "")]
    config: String,
}

fn main() {
    dotenv().ok();
    let opt = Opt::from_args();

    let mut settings = Settings::new(&opt.config).expect("Loaded settings");

    let env = Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, &settings.logging);
    env_logger::init_from_env(env);

    // Verify settings
    if !settings.verify() {
        error!("Settings are not valid");
        process::exit(1);
    }

    if let Err(error) = Application::new().run(&mut settings) {
        error!("{}", error);
        process::exit(1);
    }

} 
