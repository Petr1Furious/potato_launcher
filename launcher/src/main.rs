#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod auth;
mod config;
mod constants;
mod lang;
mod launcher;
mod message_provider;
mod update_app;
mod utils;
mod version;

use config::runtime_config;

fn main() {
    env_logger::init();

    utils::set_sigint_handler();

    let config = runtime_config::load_config();

    update_app::app::run_gui(&config);

    app::app::run_gui(config);
}
