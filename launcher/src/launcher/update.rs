use futures::StreamExt as _;
use reqwest::Client;
use std::process::Command;
use std::sync::Arc;
use std::{env, fs};

use crate::config::build_config;
use crate::lang::LangMessage;
use crate::utils;
use shared::progress::ProgressBar;

#[cfg(target_os = "windows")]
lazy_static::lazy_static! {
    static ref VERSION_URL: Option<String> = build_config::get_auto_update_base().map(|url| format!("{url}/version_windows.txt"));
}
#[cfg(target_os = "linux")]
lazy_static::lazy_static! {
    static ref VERSION_URL: Option<String> = build_config::get_auto_update_base().map(|url| format!("{url}/version_linux.txt"));
}
#[cfg(target_os = "macos")]
lazy_static::lazy_static! {
    static ref VERSION_URL: Option<String> = build_config::get_auto_update_base().map(|url| format!("{url}/version_macos.txt"));
}

#[cfg(target_os = "windows")]
lazy_static::lazy_static! {
    static ref LAUNCHER_FILE_NAME: String = format!("{}.exe", build_config::get_launcher_name());
}
#[cfg(target_os = "linux")]
lazy_static::lazy_static! {
    static ref LAUNCHER_FILE_NAME: String = format!("{}", build_config::get_data_launcher_name());
}
#[cfg(target_os = "macos")]
lazy_static::lazy_static! {
    static ref LAUNCHER_FILE_NAME: String = format!("{}_macos.tar.gz", build_config::get_data_launcher_name());
}

lazy_static::lazy_static! {
    static ref UPDATE_URL: Option<String> = build_config::get_auto_update_base().map(|url| format!("{url}/{}", &*LAUNCHER_FILE_NAME));
}

#[derive(thiserror::Error, Debug)]
pub enum UpdateError {
    #[error("Auto update URL not set")]
    AutoUpdateUrlNotSet,
}

async fn fetch_new_version() -> anyhow::Result<String> {
    if let Some(version_url) = &*VERSION_URL {
        let client = Client::new();
        let response = client.get(version_url).send().await?.error_for_status()?;
        let text = response.text().await?;
        Ok(text.trim().to_string())
    } else {
        Err(UpdateError::AutoUpdateUrlNotSet.into())
    }
}

pub async fn need_update() -> anyhow::Result<bool> {
    let new_version = fetch_new_version().await?;
    let current_version = build_config::get_version().expect("Version not set");
    Ok(new_version != current_version)
}

pub async fn download_new_launcher(
    progress_bar: Arc<dyn ProgressBar<LangMessage> + Send + Sync>,
) -> anyhow::Result<Vec<u8>> {
    if UPDATE_URL.is_none() {
        return Err(UpdateError::AutoUpdateUrlNotSet.into());
    }
    let update_url = UPDATE_URL.as_ref().unwrap();

    let client = Client::new();
    let response = client.get(update_url).send().await?.error_for_status()?;

    let total_size = response.content_length().unwrap_or(0);
    progress_bar.set_length(total_size);
    progress_bar.set_message(LangMessage::DownloadingUpdate);

    let mut bytes = Vec::with_capacity(total_size as usize);
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        bytes.extend_from_slice(&chunk);
        progress_bar.inc(chunk.len() as u64);
    }
    progress_bar.finish();

    Ok(bytes)
}

#[cfg(target_os = "macos")]
fn unarchive_tar_gz(archive_data: &[u8], dest_dir: &std::path::Path) -> std::io::Result<()> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    if dest_dir.exists() {
        fs::remove_dir_all(dest_dir)?;
    }

    fs::create_dir_all(dest_dir)?;

    let tar = GzDecoder::new(archive_data);
    let mut archive = Archive::new(tar);
    archive.unpack(dest_dir)?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn replace_launcher_and_start(new_binary: &[u8]) -> anyhow::Result<()> {
    let current_exe = env::current_exe()?;

    let new_exe = utils::get_temp_dir().join("new_launcher");
    fs::write(&new_exe, new_binary)?;
    self_replace::self_replace(&new_exe)?;
    fs::remove_file(&new_exe)?;

    let args: Vec<String> = env::args().collect();
    Command::new(&current_exe).args(&args[1..]).spawn()?;
    std::process::exit(0);
}

#[cfg(target_os = "macos")]
pub fn replace_launcher_and_start(new_archive: &[u8]) -> anyhow::Result<()> {
    let current_exe = env::current_exe()?;
    let current_dir = current_exe
        .parent()
        .expect("Failed to get current executable directory");
    let contents_dir = current_dir
        .parent()
        .expect("Failed to get Contents directory");
    let bundle_dir = contents_dir
        .parent()
        .expect("Failed to get bundle directory");

    let app_name = bundle_dir.file_name().unwrap().to_str().unwrap();

    if !app_name.ends_with(".app") {
        return Err(anyhow::Error::msg(format!(
            "Invalid bundle directory: {bundle_dir:?}",
        )));
    }

    let temp_dir = utils::get_temp_dir().join("launcher_update");
    let backup_dir = utils::get_temp_dir().join("launcher_backup");

    fs::create_dir_all(&temp_dir)?;
    fs::create_dir_all(&backup_dir)?;

    unarchive_tar_gz(new_archive, &temp_dir)?;

    if backup_dir.exists() {
        fs::remove_dir_all(&backup_dir)?;
    }

    // update.app is the name of the app bundle in the tar.gz created in ci
    const UPDATE_APP_NAME: &str = "update.app";

    fs::rename(bundle_dir, &backup_dir)?;
    fs::rename(temp_dir.join(UPDATE_APP_NAME), bundle_dir)?;
    fs::remove_dir_all(&backup_dir)?;

    let args: Vec<String> = env::args().collect();
    Command::new(&current_exe).args(&args[1..]).spawn()?;
    std::process::exit(0);
}
