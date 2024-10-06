use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::build_config;
use crate::{auth, constants, lang::Lang};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub token: Option<String>,
    pub user_info: Option<auth::base::UserInfo>,
    pub java_paths: HashMap<String, String>,
    pub assets_dir: Option<String>,
    pub data_dir: Option<String>,
    pub xmx: String,
    pub selected_modpack_name: Option<String>,
    pub lang: Lang,
    pub close_launcher_after_launch: bool,
}

pub fn get_launcher_dir(config: &Config) -> PathBuf {
    let data_dir = match &config.data_dir {
        None => dirs::data_dir()
            .expect("Failed to get data directory")
            .join(build_config::get_launcher_name()),

        Some(dir) => PathBuf::from(dir),
    };
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir).expect("Failed to create launcher directory");
    }
    data_dir
}

fn get_config_path() -> PathBuf {
    let data_dir = dirs::data_dir().expect("Failed to get data directory");
    data_dir
        .join(build_config::get_launcher_name())
        .join("config.json")
}

pub fn get_assets_dir(config: &Config) -> PathBuf {
    let assets_dir = match &config.assets_dir {
        Some(dir) => PathBuf::from(dir),
        None => get_launcher_dir(config).join("assets"),
    };
    if !assets_dir.exists() {
        std::fs::create_dir_all(&assets_dir).expect("Failed to create assets directory");
    }
    assets_dir
}

pub fn load_config() -> Config {
    let config_path = get_config_path();
    if config_path.exists() {
        let config_str = std::fs::read_to_string(&config_path).expect("Failed to read config file");
        if let Ok(config) = serde_json::from_str(&config_str) {
            return config;
        }
    }

    let config = Config {
        token: None,
        user_info: None,
        java_paths: HashMap::new(),
        assets_dir: None,
        data_dir: None,
        xmx: String::from(constants::DEFAULT_JAVA_XMX),
        selected_modpack_name: None,
        lang: constants::DEFAULT_LANG,
        close_launcher_after_launch: true,
    };
    return config;
}

pub fn save_config(config: &Config) {
    let config_str = serde_json::to_string_pretty(config).expect("Failed to serialize config");
    let config_path = get_config_path();
    std::fs::write(&config_path, config_str).expect("Failed to write config file");
}
