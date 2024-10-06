use std::{
    error::Error,
    path::{Path, PathBuf},
};

use log::info;

pub async fn exec_custom_command(command: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("Executing command: {}", command);
    let mut cmd = tokio::process::Command::new("bash");
    cmd.args(vec!["-c", command]);
    let status = cmd.status().await?;
    if !status.success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Command failed",
        )));
    }
    Ok(())
}

pub fn get_url_from_path(
    path: &Path,
    base_dir: &Path,
    download_server_base: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    Ok(format!(
        "{}/{}",
        download_server_base,
        path.strip_prefix(base_dir)?.to_string_lossy()
    ))
}

pub fn get_assets_dir(output_dir: &Path) -> PathBuf {
    let assets_dir = output_dir.join("assets");
    if !assets_dir.exists() {
        std::fs::create_dir_all(&assets_dir).unwrap();
    }
    assets_dir
}
