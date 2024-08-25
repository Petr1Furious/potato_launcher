mod app;
mod auth;
mod config;
mod constants;
mod interactive;
mod lang;
mod launcher;
mod message_provider;
mod modpack;
mod progress;
mod utils;

use config::runtime_config;
use tokio;

fn main() {
    utils::set_sigint_handler();

    let update_runtime = tokio::runtime::Runtime::new().unwrap();
    if let Err(e) = update_runtime.block_on(launcher::update::auto_update()) {
        eprintln!("Error updating: {}", e);
    }

    let config = runtime_config::load_config();
    app::app::run_gui(config);
}
