use reqwest::Client;
use sha1::{Digest, Sha1};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

use crate::config::build_config;

#[cfg(target_os = "windows")]
lazy_static::lazy_static! {
    static ref LAUNCHER_BINARY_NAME: String = format!("{}.exe", build_config::get_launcher_name());
}
#[cfg(target_os = "linux")]
lazy_static::lazy_static! {
    static ref LAUNCHER_BINARY_NAME: String = format!("{}_linux", build_config::get_launcher_name());
}
#[cfg(target_os = "macos")]
lazy_static::lazy_static! {
    static ref LAUNCHER_BINARY_NAME: String = format!("{}_macos", build_config::get_launcher_name());
}

lazy_static::lazy_static! {
    static ref UPDATE_URL: String = format!("{}/launcher/{}", build_config::get_server_base(), *LAUNCHER_BINARY_NAME);
}

async fn fetch_new_binary() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let client = Client::new();
    let response = client
        .get(UPDATE_URL.as_str())
        .send()
        .await?
        .error_for_status()?;
    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}

async fn fetch_new_hash() -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let hash_url = format!("{}.sha1", UPDATE_URL.as_str());
    let response = client
        .get(hash_url.as_str())
        .send()
        .await?
        .error_for_status()?;
    let text = response.text().await?;
    Ok(text.trim().to_string())
}

fn calculate_hash(data: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

async fn different_binaries() -> Result<bool, Box<dyn std::error::Error>> {
    let current_exe = env::current_exe()?;
    let new_hash = fetch_new_hash().await?;
    let current_binary = fs::read(&current_exe)?;
    let current_hash = calculate_hash(&current_binary);
    Ok(new_hash != current_hash)
}

#[cfg(not(target_os = "windows"))]
fn replace_binary(current_path: &Path, new_binary: &[u8]) -> std::io::Result<()> {
    use super::compat::chmod_x;

    fs::write(current_path, new_binary)?;
    chmod_x(current_path);
    Ok(())
}

#[cfg(target_os = "windows")]
fn replace_binary(current_path: &Path, new_binary: &[u8]) -> std::io::Result<()> {
    let temp_path = current_path.with_extension("tmp");
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

pub async fn auto_update() -> Result<(), Box<dyn std::error::Error>> {
    if env::var("CARGO").is_ok() {
        println!("Running from cargo, skipping auto-update");
        return Ok(());
    }

    let current_exe = env::current_exe()?;
    let new_binary = fetch_new_binary().await?;

    if different_binaries().await? {
        replace_binary(&current_exe, &new_binary)?;
        let args: Vec<String> = env::args().collect();
        let mut new_process = Command::new(&current_exe)
            .args(&args[1..])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;
        new_process.wait()?;
        std::process::exit(0);
    }

    Ok(())
}
