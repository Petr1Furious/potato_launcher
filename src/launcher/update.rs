use flate2::read::GzDecoder;
use futures::StreamExt as _;
use reqwest::Client;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::{env, fs};
use tar::Archive;

use crate::config::build_config;
use crate::lang::LangMessage;
use crate::progress::ProgressBar;
use crate::utils;

lazy_static::lazy_static! {
    static ref VERSION_URL: String = format!("{}/launcher/version.txt", build_config::get_server_base());
}

#[cfg(target_os = "windows")]
lazy_static::lazy_static! {
    static ref LAUNCHER_FILE_NAME: String = format!("{}.exe", build_config::get_display_launcher_name());
}
#[cfg(target_os = "linux")]
lazy_static::lazy_static! {
    static ref LAUNCHER_FILE_NAME: String = format!("{}", build_config::get_display_launcher_name());
}
#[cfg(target_os = "macos")]
lazy_static::lazy_static! {
    static ref LAUNCHER_FILE_NAME: String = format!("{}_macos.tar.gz", build_config::get_launcher_name());
}

lazy_static::lazy_static! {
    static ref UPDATE_URL: String = format!("{}/launcher/{}", build_config::get_server_base(), *LAUNCHER_FILE_NAME);
}

async fn fetch_new_version() -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client.get(&*VERSION_URL).send().await?.error_for_status()?;
    let text = response.text().await?;
    Ok(text.trim().to_string())
}

pub async fn need_update() -> Result<bool, Box<dyn std::error::Error>> {
    let new_version = fetch_new_version().await?;
    let current_version = build_config::get_version().expect("Version not set");
    Ok(new_version != current_version)
}

pub async fn download_new_launcher(
    progress_bar: Arc<dyn ProgressBar + Send + Sync>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client
        .get(UPDATE_URL.as_str())
        .send()
        .await?
        .error_for_status()?;

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
fn unarchive_tar_gz(archive_data: &[u8], dest_dir: &Path) -> std::io::Result<()> {
    if dest_dir.exists() {
        fs::remove_dir_all(dest_dir)?;
    }

    fs::create_dir_all(dest_dir)?;

    let tar = GzDecoder::new(archive_data);
    let mut archive = Archive::new(tar);
    archive.unpack(dest_dir)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn replace_binary(current_exe: &Path, new_binary: &[u8]) -> std::io::Result<()> {
    use super::compat::chmod_x;

    let temp_path =
        utils::get_temp_dir().join(format!("{}-new", build_config::get_launcher_name()));
    fs::write(&temp_path, new_binary)?;
    chmod_x(&temp_path)?;
    fs::rename(temp_path, current_exe)?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn replace_binary(current_path: &Path, new_binary: &[u8]) -> std::io::Result<()> {
    let temp_path =
        utils::get_temp_dir().join(format!("{}-new.exe", build_config::get_launcher_name()));
    fs::write(&temp_path, new_binary)?;
    Command::new("cmd")
        .args(&[
            "/C",
            "move",
            "/Y",
            temp_path.to_str().unwrap(),
            current_path.to_str().unwrap(),
        ])
        .spawn()?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn replace_launcher_and_start(new_binary: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let current_exe = env::current_exe()?;
    replace_binary(&current_exe, &new_binary)?;
    let args: Vec<String> = env::args().collect();

    Command::new(&current_exe).args(&args[1..]).spawn()?;
    std::process::exit(0);
}

#[cfg(target_os = "macos")]
pub fn replace_launcher_and_start(new_archive: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
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
        return Err(format!("Invalid bundle directory: {:?}", bundle_dir).into());
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

    fs::rename(&bundle_dir, &backup_dir)?;
    fs::rename(&temp_dir.join(UPDATE_APP_NAME), &bundle_dir)?;
    fs::remove_dir_all(&backup_dir)?;

    let args: Vec<String> = env::args().collect();
    Command::new(&current_exe).args(&args[1..]).spawn()?;
    std::process::exit(0);
}
