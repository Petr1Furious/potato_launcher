use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::config::build_config;
use crate::config::runtime_config;
use crate::lang::get_loc;
use crate::progress::ProgressBar;

use super::files::get_files_in_dir;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModpackIndex {
    pub modpack_name: String,
    pub java_version: String,
    pub minecraft_version: String,
    pub modpack_version: String,
    pub asset_index: String,
    pub main_class: String,
    pub libraries: Vec<serde_json::Value>,
    pub java_args: Vec<serde_json::Value>,
    pub game_args: Vec<serde_json::Value>,
    pub include: Vec<String>,
    pub include_no_overwrite: Vec<String>,
    pub objects: HashMap<String, String>,
    pub client_filename: String,
}

pub async fn load_remote_indexes() -> Result<Vec<ModpackIndex>, Box<dyn std::error::Error>> {
    let client = Client::new();
    let res = client
        .get(format!("{}/index.json", build_config::get_server_base()))
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<ModpackIndex>>()
        .await?;
    Ok(res)
}

pub fn load_local_indexes(config: &runtime_config::Config) -> Vec<ModpackIndex> {
    let index_path = runtime_config::get_index_path(config);
    if !index_path.is_file() {
        return vec![];
    }
    match fs::read_to_string(index_path) {
        Ok(data) => match serde_json::from_str(&data) {
            Ok(indexes) => indexes,
            Err(_) => vec![],
        },
        Err(_) => vec![],
    }
}

pub fn get_local_index(config: &runtime_config::Config) -> Option<ModpackIndex> {
    let indexes = load_local_indexes(config);
    indexes
        .into_iter()
        .find(|x| &x.modpack_name == config.modpack_name.as_ref().unwrap())
}

fn save_local_index(config: &runtime_config::Config, index: ModpackIndex) {
    let mut indexes = load_local_indexes(config);
    indexes.retain(|x| x.modpack_name != index.modpack_name);
    indexes.push(index);
    if let Ok(data) = serde_json::to_string_pretty(&indexes) {
        let _ = fs::write(runtime_config::get_index_path(config), data);
    }
}

pub async fn sync_modpack(
    config: &runtime_config::Config,
    index: ModpackIndex,
    force_overwrite: bool,
    progress_bar: Arc<dyn ProgressBar + Send + Sync>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    clearscreen::clear().unwrap();

    let modpack_dir = runtime_config::get_minecraft_dir(config, &index.modpack_name);
    let assets_dir = runtime_config::get_assets_dir(config);

    let get_modpack_files = |x| get_files_in_dir(&modpack_dir.join(x));
    let no_overwrite_iter = index
        .include_no_overwrite
        .iter()
        .map(get_modpack_files)
        .flatten();
    let assets_iter = get_files_in_dir(&assets_dir).into_iter();

    let mut abs_path_overwrite: HashSet<PathBuf> = index
        .include
        .iter()
        .map(get_modpack_files)
        .flatten()
        .collect();
    let mut abs_path_no_overwrite = HashSet::new();
    if !force_overwrite {
        abs_path_no_overwrite.extend(no_overwrite_iter);
        abs_path_no_overwrite.extend(assets_iter);
    } else {
        abs_path_overwrite.extend(no_overwrite_iter);
        abs_path_overwrite.extend(assets_iter);
    }

    progress_bar.set_message(get_loc(&config.lang).checking_files);
    let abs_path_overwrite_hashes = super::files::hash_files(abs_path_overwrite.clone().into_iter(), Arc::clone(&progress_bar)).await?;
    let mut urls: Vec<String> = vec![];
    let mut paths: Vec<PathBuf> = vec![];

    for path in abs_path_overwrite.iter() {
        let file = if path.starts_with(&modpack_dir) {
            path.strip_prefix(&modpack_dir)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        } else {
            format!(
                "assets/{}",
                path.strip_prefix(&assets_dir).unwrap().to_str().unwrap()
            )
        };
        if !index.objects.contains_key(&file) {
            fs::remove_file(path)?;
        }
    }
    for (file, remote_hash) in index.objects.iter() {
        let path = if file.starts_with("assets/") {
            assets_dir.join(&file.strip_prefix("assets/").unwrap())
        } else {
            modpack_dir.join(file)
        };

        if abs_path_no_overwrite.contains(&path) {
            continue;
        }
        let need_download: bool;
        match abs_path_overwrite_hashes.get(&path) {
            Some(hash) => need_download = hash != remote_hash,
            None => need_download = true,
        }
        if need_download {
            urls.push(format!(
                "{}/{}/{}",
                build_config::get_server_base(),
                index.modpack_name,
                file
            ));
            paths.push(path);
        }
    }

    progress_bar.set_message(get_loc(&config.lang).downloading_files);
    super::files::download_files(urls.into_iter(), paths.into_iter(), progress_bar).await?;

    save_local_index(config, index);
    Ok(())
}