use std::path::Path;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::utils::BoxResult;

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

    pub fn get_parent_metadata_info(&self) -> MetadataInfo {
        match self.inherits_from.get(0) {
            Some(parent_info) => parent_info.clone(),
            None => MetadataInfo {
                id: self.id.clone(),
                url: self.url.clone(),
                sha1: self.sha1.clone(),
            },
        }
    }

    pub fn get_metadata_info(&self) -> Vec<MetadataInfo> {
        let mut versions_info = vec![self.get_parent_metadata_info()];
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

#[derive(Serialize, Deserialize, Clone)]
pub struct VersionManifest {
    pub versions: Vec<VersionInfo>,
}

impl VersionManifest {
    pub fn empty() -> Self {
        Self {
            versions: Vec::new(),
        }
    }

    pub async fn fetch(url: &str) -> BoxResult<Self> {
        let client = Client::new();
        let res = client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json::<Self>()
            .await?;
        Ok(res)
    }

    pub async fn read_local(manifest_path: &Path) -> BoxResult<Self> {
        let manifest_file = tokio::fs::read(manifest_path).await?;
        let manifest: Self = serde_json::from_slice(&manifest_file)?;
        Ok(manifest)
    }

    pub async fn read_local_safe(manifest_path: &Path) -> Self {
        match Self::read_local(manifest_path).await {
            Ok(manifest) => manifest,
            Err(_) => Self {
                versions: Vec::new(),
            },
        }
    }

    pub async fn save_to_file(&self, manifest_path: &Path) -> BoxResult<()> {
        let manifest_str = serde_json::to_string(self)?;
        tokio::fs::write(manifest_path, manifest_str).await?;
        Ok(())
    }

    pub fn is_up_to_date(&self, version_info: &VersionInfo) -> bool {
        self.versions.iter().find(|i| i == &version_info).is_some()
    }

    pub async fn add_version_and_save(
        &mut self,
        version_info: VersionInfo,
        manifest_path: &Path,
    ) -> BoxResult<()> {
        self.versions
            .retain(|i| i.get_name() != version_info.get_name());
        self.versions.push(version_info);
        self.save_to_file(manifest_path).await
    }
}
