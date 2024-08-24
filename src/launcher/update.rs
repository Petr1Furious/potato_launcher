use reqwest::Client;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::config::build_config;

lazy_static::lazy_static! {
    static ref UPDATE_URL: String = format!("{}/launcher/{}", build_config::get_server_base(), build_config::get_launcher_name());
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

async fn compare_binaries() -> Result<bool, Box<dyn std::error::Error>> {
    let current_exe = env::current_exe().unwrap();
    let new_hash = fetch_new_hash().await?;
    let current_hash = fs::read(&current_exe).unwrap();
    Ok(new_hash.as_bytes() != current_hash.as_slice())
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
    let current_exe = env::current_exe()?;
    let new_binary = fetch_new_binary().await?;

    if !compare_binaries().await? {
        replace_binary(&current_exe, &new_binary)?;
        let args: Vec<String> = env::args().collect();
        Command::new(&current_exe).args(&args[1..]).spawn()?;
        std::process::exit(0);
    }

    Ok(())
}
