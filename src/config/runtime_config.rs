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
    pub modpack_name: Option<String>,
    pub lang: Lang,
}

fn get_data_dir(config: &Config) -> PathBuf {
    let data_dir = match &config.data_dir {
        None => dirs::data_dir()
            .expect("Failed to get data directory")
            .join(build_config::get_launcher_name()),

        Some(dir) => PathBuf::from(dir),
    };
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");
    }
    data_dir
}

pub fn get_assets_dir(config: &Config) -> PathBuf {
    let assets_dir = match &config.assets_dir {
        None => get_data_dir(config).join("assets"),

        Some(dir) => PathBuf::from(dir),
    };
    if !assets_dir.exists() {
        std::fs::create_dir_all(&assets_dir).expect("Failed to create assets directory");
    }
    assets_dir
}

pub fn get_minecraft_dir(config: &Config, modpack_name: &String) -> PathBuf {
    let minecraft_dir = get_data_dir(config).join("modpacks").join(modpack_name);
    if !minecraft_dir.exists() {
        std::fs::create_dir_all(&minecraft_dir).expect("Failed to create minecraft directory");
    }
    minecraft_dir
}

pub fn get_index_path(config: &Config) -> PathBuf {
    let modpacks_path = get_data_dir(config).join("modpacks");
    if !modpacks_path.exists() {
        std::fs::create_dir_all(&modpacks_path).expect("Failed to create modpacks directory");
    }
    modpacks_path.join("index.json")
}

fn get_config_path() -> PathBuf {
    let config_dir = dirs::config_dir().expect("Failed to get config directory").join(build_config::get_launcher_name());
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir).expect("Failed to create config directory");
    }
    config_dir.join("config.json")
}

pub fn get_java_dir(config: &Config) -> PathBuf {
    let java_dir = get_data_dir(config).join("java");
    if !java_dir.exists() {
        std::fs::create_dir_all(&java_dir).expect("Failed to create java directory");
    }
    java_dir
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
        modpack_name: None,
        lang: constants::DEFAULT_LANG,
    };
    return config;
}

pub fn save_config(config: &Config) {
    let config_str = serde_json::to_string_pretty(config).expect("Failed to serialize config");
    let config_path = get_config_path();
    std::fs::write(&config_path, config_str).expect("Failed to write config file");
}
