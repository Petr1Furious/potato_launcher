use std::path::Path;

use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct MetadataInfo {
    pub id: String,
    pub url: String,
    pub sha1: String,
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionInfo {
    pub id: String,
    pub url: String,
    pub sha1: String,

    pub name: Option<String>,

    #[serde(default)]
    pub inherits_from: Vec<MetadataInfo>,

    pub extra_metadata_url: Option<String>,
    pub extra_metadata_sha1: Option<String>,
}

impl VersionInfo {
    pub fn get_name(&self) -> String {
        match &self.name {
            Some(name) => name.clone(),
            None => self.id.clone(),
        }
    }

    pub fn get_metadata_info(&self) -> Vec<MetadataInfo> {
        let mut versions_info = Vec::new();
        for version_info in &self.inherits_from {
            versions_info.push(version_info.clone());
        }
        versions_info.push(MetadataInfo {
            id: self.id.clone(),
            url: self.url.clone(),
            sha1: self.sha1.clone(),
        });
        versions_info
    }
}

#[derive(Serialize, Deserialize)]
pub struct VersionManifest {
    pub versions: Vec<VersionInfo>,
}

pub async fn fetch_version_manifest(
    version_manifest_url: &str,
) -> Result<VersionManifest, Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::new();
    let res = client
        .get(version_manifest_url)
        .send()
        .await?
        .error_for_status()?
        .json::<VersionManifest>()
        .await?;
    Ok(res)
}

pub async fn load_local_version_manifest(
    manifest_path: &Path,
) -> Result<VersionManifest, Box<dyn std::error::Error + Send + Sync>> {
    let manifest_file = tokio::fs::read(manifest_path).await?;
    let manifest: VersionManifest = serde_json::from_slice(&manifest_file)?;
    Ok(manifest)
}

pub async fn load_local_version_manifest_safe(manifest_path: &Path) -> VersionManifest {
    match load_local_version_manifest(manifest_path).await {
        Ok(manifest) => manifest,
        Err(_) => VersionManifest {
            versions: Vec::new(),
        },
    }
}

pub async fn save_local_version_manifest(
    manifest: &VersionManifest,
    manifest_path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manifest_str = serde_json::to_string(manifest)?;
    tokio::fs::write(manifest_path, manifest_str).await?;
    Ok(())
}
