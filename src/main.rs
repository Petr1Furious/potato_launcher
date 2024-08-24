mod app;
mod auth;
mod config;
mod constants;
mod interactive;
mod lang;
mod launcher;
mod modpack;
mod progress;
mod utils;
mod message_provider;

use config::runtime_config;
use tokio;

fn main() {
    utils::set_sigint_handler();

    let update_runtime = tokio::runtime::Runtime::new().unwrap();
    if let Err(e) = update_runtime.block_on(launcher::update::auto_update()) {
        eprintln!("Error updating: {}", e);
    }

    // let progress_bar = Arc::new(progress::TerminalBarWrapper::new());

    // let selected_index = indexes.iter().find(|x| &x.modpack_name == config.modpack_name.as_ref().unwrap()).unwrap().clone();
    // if online {
    //     let local_index = modpack::index::get_local_index(&config);
    //     if (local_index.is_some() && local_index.as_ref().unwrap().modpack_version != selected_index.modpack_version) || local_index.is_none() {
    //         sync_modpack(&config, selected_index.clone(), false, progress_bar.clone()).await.unwrap();
    //     }
    // }

    // launcher::launch::launch(selected_index, &config, online).await.unwrap();

    let config = runtime_config::load_config();
    app::app::run_gui(config);
}
