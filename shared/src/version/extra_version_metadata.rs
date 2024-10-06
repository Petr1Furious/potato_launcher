use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::files;

use super::{version_manifest::VersionInfo, version_metadata::Download};

#[derive(Deserialize, Serialize)]
pub struct Object {
    pub path: String,
    pub sha1: String,
    pub url: String,
}

#[derive(Deserialize, Serialize)]
pub struct ExtraVersionMetadata {
    pub version_name: String,
    pub include: Vec<String>,
    pub include_no_overwrite: Vec<String>,
    pub objects: Vec<Object>,
    pub resources_url_base: Option<String>,
    pub client_override: Option<Download>,
}

pub fn get_extra_version_metadata_path(versions_extra_dir: &Path, version_name: &str) -> PathBuf {
    versions_extra_dir.join(format!("{}.json", version_name))
}

pub async fn read_local_extra_version_metadata(
    version_info: &VersionInfo,
    versions_extra_dir: &Path,
) -> Result<Option<ExtraVersionMetadata>, Box<dyn std::error::Error + Send + Sync>> {
    if version_info.extra_metadata_url.is_none() || version_info.extra_metadata_sha1.is_none() {
        return Ok(None);
    }

    let extra_version_metadata_path =
        get_extra_version_metadata_path(versions_extra_dir, &version_info.get_name());
    let extra_version_metadata_file = tokio::fs::read(extra_version_metadata_path).await?;
    let extra_version_metadata: ExtraVersionMetadata =
        serde_json::from_slice(&extra_version_metadata_file)?;

    Ok(Some(extra_version_metadata))
}

pub async fn get_extra_version_metadata(
    version_info: &VersionInfo,
    versions_extra_dir: &Path,
) -> Result<Option<ExtraVersionMetadata>, Box<dyn std::error::Error + Send + Sync>> {
    if version_info.extra_metadata_url.is_none() || version_info.extra_metadata_sha1.is_none() {
        return Ok(None);
    }

    let url = version_info.extra_metadata_url.as_ref().unwrap();
    let sha1 = version_info.extra_metadata_sha1.as_ref().unwrap();

    let extra_version_metadata_path =
        get_extra_version_metadata_path(versions_extra_dir, &version_info.get_name());
    let need_download;
    if !extra_version_metadata_path.exists() {
        need_download = true;
    } else {
        let local_sha1 = files::hash_file(&extra_version_metadata_path).await?;
        need_download = &local_sha1 != sha1;
    }

    if need_download {
        let client = reqwest::Client::new();
        files::download_file(&client, url, &extra_version_metadata_path).await?;
    }

    Ok(read_local_extra_version_metadata(version_info, versions_extra_dir).await?)
}

pub async fn save_extra_version_metadata(
    versions_extra_dir: &Path,
    version_name: &str,
    extra_version_metadata: &ExtraVersionMetadata,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let extra_version_metadata_path =
        get_extra_version_metadata_path(versions_extra_dir, version_name);
    let extra_version_metadata_file = serde_json::to_string(extra_version_metadata)?;
    tokio::fs::write(extra_version_metadata_path, extra_version_metadata_file).await?;

    Ok(())
}
