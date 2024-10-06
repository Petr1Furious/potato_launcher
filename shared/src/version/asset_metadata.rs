use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    files::{self, CheckDownloadEntry},
    paths::get_asset_index_path,
    version::version_metadata::AssetIndex,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ObjectData {
    pub hash: String,
}

#[derive(Serialize, Deserialize)]
pub struct AssetsMetadata {
    pub objects: HashMap<String, ObjectData>,
}

impl AssetsMetadata {
    pub async fn fetch(
        url: &str,
    ) -> Result<AssetsMetadata, Box<dyn std::error::Error + Send + Sync>> {
        let client = Client::new();
        let response = client.get(url).send().await?.json().await?;
        Ok(response)
    }

    async fn get_path(assets_dir: &Path, asset_id: &str) -> Result<PathBuf, std::io::Error> {
        tokio::fs::create_dir_all(assets_dir.join("indexes")).await?;
        Ok(assets_dir
            .join("indexes")
            .join(format!("{}.json", asset_id)))
    }

    pub async fn read(
        asset_id: &str,
        assets_dir: &Path,
    ) -> Result<AssetsMetadata, Box<dyn std::error::Error + Send + Sync>> {
        let data = tokio::fs::read(AssetsMetadata::get_path(&assets_dir, asset_id).await?).await?;
        let data: AssetsMetadata = serde_json::from_slice(&data)?;
        Ok(data)
    }

    pub async fn read_or_fetch(
        asset_index: &AssetIndex,
        assets_dir: &Path,
    ) -> Result<AssetsMetadata, Box<dyn std::error::Error + Send + Sync>> {
        let mut needed_index_download = false;

        let asset_index_path = get_asset_index_path(assets_dir, &asset_index.id);
        if !asset_index_path.exists() {
            needed_index_download = true;
        } else {
            let local_asset_index_hash = files::hash_file(&asset_index_path).await?;
            if local_asset_index_hash != asset_index.sha1 {
                needed_index_download = true;
            }
        }

        Ok(if needed_index_download {
            AssetsMetadata::fetch(&asset_index.url).await?
        } else {
            AssetsMetadata::read(&asset_index.id, &assets_dir).await?
        })
    }

    pub fn get_check_downloads(
        &self,
        assets_dir: &Path,
        resources_url_base: &str,
    ) -> Result<Vec<CheckDownloadEntry>, Box<dyn std::error::Error + Send + Sync>> {
        let mut download_entries = vec![];

        download_entries.extend(self.objects.iter().map(|(_, object)| {
            CheckDownloadEntry {
                url: format!(
                    "{}/{}/{}",
                    resources_url_base,
                    &object.hash[..2],
                    object.hash
                ),
                path: assets_dir
                    .join("objects")
                    .join(&object.hash[..2])
                    .join(&object.hash),
                remote_sha1: None, // do not check sha1 for assets since it's in the path
            }
        }));

        Ok(download_entries)
    }

    pub async fn save_to_file(
        &self,
        asset_id: &str,
        assets_dir: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let data = serde_json::to_vec(self)?;
        tokio::fs::write(AssetsMetadata::get_path(&assets_dir, asset_id).await?, data).await?;
        Ok(())
    }
}
