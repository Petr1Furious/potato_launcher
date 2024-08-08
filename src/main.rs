mod config;
mod auth;
mod constants;
mod lang;
mod utils;
mod modpack;
mod interactive;
mod launcher;
mod progress;

use std::sync::Arc;

use config::runtime_config;
use modpack::index::{sync_modpack, ModpackIndex};
use tokio;

#[tokio::main]
async fn main() {
    let mut config = runtime_config::load_config();
    utils::set_sigint_handler(&config);

    let mut online = auth::auth::auth_and_save(&mut config).await;
    
    let mut indexes: Vec<ModpackIndex> = vec![];
    if online {
        match modpack::index::load_remote_indexes().await {
            Ok(i) => indexes = i,
            Err(_) => online = false,
        }
    }
    if !online {
        indexes = modpack::index::load_local_indexes(&config)
    };

    if indexes.is_empty() {
        utils::print_error_and_exit(if online {
            lang::get_loc(&config.lang).no_remote_modpacks
        } else {
            lang::get_loc(&config.lang).no_local_modpacks
        });
    }

    if config.modpack_name.is_none() || !indexes.iter().any(|x| &x.modpack_name == config.modpack_name.as_ref().unwrap()) {
        let modpack_name = interactive::select_modpack(&config, &indexes);
        config.modpack_name = Some(modpack_name);
        runtime_config::save_config(&config);
    }

    let progress_bar = Arc::new(progress::TerminalBarWrapper::new());

    let selected_index = indexes.iter().find(|x| &x.modpack_name == config.modpack_name.as_ref().unwrap()).unwrap().clone();
    if online {
        let local_index = modpack::index::get_local_index(&config);
        if (local_index.is_some() && local_index.as_ref().unwrap().modpack_version != selected_index.modpack_version) || local_index.is_none() {
            sync_modpack(&config, selected_index.clone(), false, progress_bar.clone()).await.unwrap();
        }
    }

    let mut installation = launcher::java::get_java(selected_index.java_version.as_str(), &runtime_config::get_java_dir(&config));
    if installation.is_none() {
        installation = Some(launcher::java::download_java(selected_index.java_version.as_str(), &runtime_config::get_java_dir(&config), progress_bar, &config.lang).await.unwrap());
    }
    println!("Java installation: {:?}", installation);
}
