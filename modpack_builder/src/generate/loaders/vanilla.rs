use std::{error::Error, path::Path};

use async_trait::async_trait;
use log::info;
use reqwest::Client;
use shared::{
    files::download_file,
    paths::get_versions_dir,
    version::{
        version_manifest::{fetch_version_manifest, VersionInfo},
        version_metadata::{
            get_version_metadata_path, read_version_metadata, save_version_metadata,
            VersionMetadata,
        },
    },
};

use crate::generate::{patch::replace_download_urls, sync::sync_version};

use super::generator::VersionGenerator;

pub struct VanillaGenerator {
    version_name: String,
    minecraft_version: String,
    download_server_base: String,
    replace_download_urls: bool,
}

impl VanillaGenerator {
    pub fn new(
        version_name: String,
        minecraft_version: String,
        download_server_base: String,
        replace_download_urls: bool,
    ) -> Self {
        Self {
            version_name,
            minecraft_version,
            download_server_base,
            replace_download_urls,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum VanillaGeneratorError {
    #[error("Vanilla version not found")]
    VersionNotFound,
}

pub async fn download_vanilla_metadata(
    version_info: &VersionInfo,
    output_dir: &Path,
) -> Result<VersionMetadata, Box<dyn Error + Send + Sync>> {
    let versions_dir = get_versions_dir(&output_dir);
    let version_metadata_path = get_version_metadata_path(&versions_dir, &version_info.id);

    let client = Client::new();
    download_file(&client, &version_info.url, &version_metadata_path).await?;
    return Ok(read_version_metadata(&versions_dir, &version_info.id).await?);
}

const VANILLA_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[async_trait]
impl VersionGenerator for VanillaGenerator {
    async fn generate(
        &self,
        output_dir: &Path,
        _: &Path,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        info!(
            "Generating vanilla version \"{}\", minecraft version {}",
            self.version_name, self.minecraft_version
        );

        info!("Fetching version manifest");
        let vanilla_manifest = fetch_version_manifest(VANILLA_MANIFEST_URL).await?;
        let version_info = vanilla_manifest
            .versions
            .iter()
            .find(|v| v.id == self.minecraft_version)
            .ok_or_else(|| VanillaGeneratorError::VersionNotFound)?;

        info!("Downloading version metadata");
        let mut vanilla_metadata = download_vanilla_metadata(version_info, output_dir).await?;

        if self.replace_download_urls {
            info!("Syncing version");
            sync_version(&vanilla_metadata, &self.version_name, output_dir).await?;

            replace_download_urls(
                &self.version_name,
                &mut vanilla_metadata,
                &self.download_server_base,
                output_dir,
            )
            .await?;

            let versions_dir = get_versions_dir(&output_dir);
            save_version_metadata(&versions_dir, &vanilla_metadata).await?;
        }

        info!("Vanilla version \"{}\" generated", self.version_name);

        Ok(vanilla_metadata.id.clone())
    }
}
