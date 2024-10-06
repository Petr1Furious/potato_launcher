use std::{error::Error, path::Path};

use async_trait::async_trait;
use log::info;
use reqwest::Client;
use serde::Deserialize;
use shared::{
    paths::get_versions_dir,
    version::version_metadata::{save_version_metadata, VersionMetadata},
};

use crate::generate::{
    loaders::vanilla::VanillaGenerator, patch::replace_download_urls, sync::sync_version,
};

use super::generator::VersionGenerator;

const FABRIC_META_BASE_URL: &str = "https://meta.fabricmc.net/v2/versions/loader/";

#[derive(Deserialize)]
struct FabricVersionLoader {
    version: String,
}

#[derive(Deserialize)]
struct FabricVersionMeta {
    loader: FabricVersionLoader,
}

pub async fn get_latest_fabric_version(
    game_version: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let fabric_manifest_url = format!("{}{}", FABRIC_META_BASE_URL, game_version);
    let client = Client::new();
    let response = client
        .get(&fabric_manifest_url)
        .send()
        .await?
        .error_for_status()?;
    let fabric_versions: Vec<FabricVersionMeta> = response.json().await?;
    return Ok(fabric_versions[0].loader.version.clone());
}

pub async fn download_fabric_metadata(
    minecraft_version: &str,
    loader_version: &str,
    output_dir: &Path,
) -> Result<VersionMetadata, Box<dyn Error + Send + Sync>> {
    let fabric_metadata_url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
        minecraft_version, loader_version
    );
    let version_metadata = VersionMetadata::from_url(&fabric_metadata_url).await?;
    let versions_dir = get_versions_dir(&output_dir);
    save_version_metadata(&versions_dir, &version_metadata).await?;
    Ok(version_metadata)
}

pub struct FabricGenerator {
    version_name: String,
    minecraft_version: String,
    loader_version: Option<String>,
    download_server_base: String,
    replace_download_urls: bool,
}

impl FabricGenerator {
    pub fn new(
        version_name: String,
        minecraft_version: String,
        loader_version: Option<String>,
        download_server_base: String,
        replace_download_urls: bool,
    ) -> Self {
        Self {
            version_name,
            minecraft_version,
            loader_version,
            download_server_base,
            replace_download_urls,
        }
    }
}

#[async_trait]
impl VersionGenerator for FabricGenerator {
    async fn generate(
        &self,
        output_dir: &Path,
        _: &Path,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        info!(
            "Generating fabric version \"{}\", minecraft version {}",
            self.version_name, self.minecraft_version
        );

        info!("Generating vanilla version first");
        let vanilla_generator = VanillaGenerator::new(
            self.version_name.clone(),
            self.minecraft_version.clone(),
            self.download_server_base.clone(),
            self.replace_download_urls,
        );
        vanilla_generator.generate(output_dir, output_dir).await?;

        let fabric_version = match &self.loader_version {
            Some(loader_version) => loader_version.clone(),
            None => {
                let version = get_latest_fabric_version(&self.minecraft_version).await?;
                info!(
                    "Loader version not specified, using latest version: {}",
                    version
                );
                version
            }
        };

        info!("Fetching fabric version metadata");
        let mut fabric_metadata =
            download_fabric_metadata(&self.minecraft_version, &fabric_version, output_dir).await?;

        if self.replace_download_urls {
            info!("Syncing version");
            sync_version(&fabric_metadata, &self.version_name, output_dir).await?;

            replace_download_urls(
                &self.version_name,
                &mut fabric_metadata,
                &self.download_server_base,
                output_dir,
            )
            .await?;

            let versions_dir = get_versions_dir(&output_dir);
            save_version_metadata(&versions_dir, &fabric_metadata).await?;

            // fabric doesn't change the client jar
            // so we don't need to do anything
        }

        info!("Fabric version \"{}\" generated", self.version_name);

        Ok(fabric_metadata.id.clone())
    }
}
