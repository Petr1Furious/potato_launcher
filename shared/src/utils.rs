use std::path::Path;
use std::env;

use crate::version::version_manifest::{VersionInfo, VersionManifest};

pub async fn exec_custom_command(command: &str) -> anyhow::Result<()> {
    exec_custom_command_in_dir(command, &Path::new(".")).await
}

pub async fn exec_custom_command_in_dir(command: &str, dir: &Path) -> anyhow::Result<()> {
    let parts = shell_words::split(command)?;
    let mut modified_parts: Vec<String> = parts.iter().map(|part| part.clone()).collect();

    // Костыль под винду для созданиия новых инстансов
    if env::consts::OS == "windows" && modified_parts[0][..3].to_string() == "\\?\\" && modified_parts[2][..3].to_string() == "\\?\\" {
        // Удаляем первые 3 символа у первого и третьего элемента
        if modified_parts.len() > 0 {
            modified_parts[0] = modified_parts[0][3..].to_string(); // Удаляем первые 3 символа у первого элемента
        }
        if modified_parts.len() > 2 {
            modified_parts[2] = modified_parts[2][3..].to_string(); // Удаляем первые 3 символа у третьего элемента
        }
    }

    let mut cmd = tokio::process::Command::new(&modified_parts[0]);
    if modified_parts.len() > 1 {
        cmd.args(&modified_parts[1..]);
    }

    cmd.current_dir(dir);
    let status = cmd.status().await?;
    if !status.success() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Command failed").into());
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
) -> anyhow::Result<VersionInfo> {
    let version_info = version_manifest
        .versions
        .iter()
        .find(|v| v.id == minecraft_version)
        .ok_or(VanillaGeneratorError::VersionNotFound)?;
    Ok(version_info.clone())
}

pub fn url_from_rel_path(rel_path: &Path, download_server_base: &str) -> anyhow::Result<String> {
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
) -> anyhow::Result<String> {
    let rel_path = path.strip_prefix(base_dir)?;
    url_from_rel_path(rel_path, download_server_base)
}
