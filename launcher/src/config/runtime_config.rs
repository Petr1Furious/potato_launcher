use serde::{Deserialize, Serialize};
use shared::{paths::get_logs_dir, version::extra_version_metadata::AuthData};
use std::collections::HashMap;
use std::path::PathBuf;

use super::build_config;
use crate::{auth::base::UserInfo, constants, lang::Lang};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct VersionAuthData {
    pub token: String,
    pub user_info: UserInfo,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub versions_auth_data: HashMap<String, VersionAuthData>,
    pub java_paths: HashMap<String, String>,
    pub assets_dir: Option<String>,
    pub data_dir: Option<String>,
    pub xmx: String,
    pub selected_instance_name: Option<String>,
    pub lang: Lang,
    pub close_launcher_after_launch: bool,
}

fn get_data_dir() -> PathBuf {
    let data_dir = dirs::data_dir()
        .expect("Failed to get data directory")
        .join(build_config::get_data_launcher_name());
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");
    }
    data_dir
}

const CONFIG_FILENAME: &str = "config.json";

fn get_config_path() -> PathBuf {
    get_data_dir().join(CONFIG_FILENAME)
}

impl Config {
    pub fn load() -> Config {
        let config_path = get_config_path();
        if config_path.exists() {
            let config_str =
                std::fs::read_to_string(&config_path).expect("Failed to read config file");
            if let Ok(config) = serde_json::from_str(&config_str) {
                return config;
            }
        }

        let config = Config {
            versions_auth_data: HashMap::new(),
            java_paths: HashMap::new(),
            assets_dir: None,
            data_dir: None,
            xmx: String::from(constants::DEFAULT_JAVA_XMX),
            selected_instance_name: None,
            lang: constants::DEFAULT_LANG,
            close_launcher_after_launch: true,
        };
        return config;
    }

    pub fn get_launcher_dir(&self) -> PathBuf {
        let data_dir = match &self.data_dir {
            None => dirs::data_dir()
                .expect("Failed to get data directory")
                .join(build_config::get_data_launcher_name()),

            Some(dir) => PathBuf::from(dir),
        };
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir).expect("Failed to create launcher directory");
        }
        data_dir
    }

    pub fn get_assets_dir(&self) -> PathBuf {
        let assets_dir = match &self.assets_dir {
            Some(dir) => PathBuf::from(dir),
            None => self.get_launcher_dir().join("assets"),
        };
        if !assets_dir.exists() {
            std::fs::create_dir_all(&assets_dir).expect("Failed to create assets directory");
        }
        assets_dir
    }

    pub fn get_version_auth_data(&self, auth_data: &AuthData) -> Option<&VersionAuthData> {
        self.versions_auth_data.get(&auth_data.get_id())
    }

    pub fn save(&self) {
        let config_str = serde_json::to_string_pretty(self).expect("Failed to serialize config");
        let config_path = get_config_path();
        std::fs::write(&config_path, config_str).expect("Failed to write config file");
    }
}

const LOGS_FILENAME: &str = "launcher.log";

pub fn get_logs_path() -> PathBuf {
    let logs_dir = get_logs_dir(&get_data_dir());
    if !logs_dir.exists() {
        std::fs::create_dir_all(&logs_dir).expect("Failed to create logs directory");
    }
    logs_dir.join(LOGS_FILENAME)
}
