use std::{error::Error, path::Path};

use crate::version::version_manifest::{VersionInfo, VersionManifest};

pub type BoxError = Box<dyn Error + Send + Sync>;

pub type BoxResult<T> = Result<T, BoxError>;

pub async fn exec_custom_command(command: &str) -> BoxResult<()> {
    exec_custom_command_in_dir(command, &Path::new(".")).await
}

pub async fn exec_custom_command_in_dir(command: &str, dir: &Path) -> BoxResult<()> {
    let mut cmd = tokio::process::Command::new("bash");
    cmd.args(vec!["-c", command]).current_dir(dir);
    let status = cmd.status().await?;
    if !status.success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Command failed",
        )));
    }
    Ok(())
}

pub const VANILLA_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(thiserror::Error, Debug)]
pub enum VanillaGeneratorError {
    #[error("Vanilla version not found")]
    VersionNotFound,
}

pub fn get_vanilla_version_info(
    version_manifest: &VersionManifest,
    minecraft_version: &str,
) -> BoxResult<VersionInfo> {
    let version_info = version_manifest
        .versions
        .iter()
        .find(|v| v.id == minecraft_version)
        .ok_or(VanillaGeneratorError::VersionNotFound)?;
    Ok(version_info.clone())
}

pub fn url_from_rel_path(rel_path: &Path, download_server_base: &str) -> BoxResult<String> {
    Ok(format!(
        "{}/{}",
        download_server_base,
        rel_path.to_string_lossy()
    ))
}

pub fn url_from_path(
    path: &Path,
    base_dir: &Path,
    download_server_base: &str,
) -> BoxResult<String> {
    let rel_path = path.strip_prefix(base_dir)?;
    url_from_rel_path(rel_path, download_server_base)
}
