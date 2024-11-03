use std::path::Path;

use crate::{
    paths::get_versions_dir,
    utils::BoxResult,
    version::{version_manifest::VersionInfo, version_metadata::VersionMetadata},
};
use async_trait::async_trait;
use log::info;
use reqwest::Client;
use serde::Deserialize;

use super::generator::{GeneratorResult, VersionGenerator};

const FABRIC_META_BASE_URL: &str = "https://meta.fabricmc.net/v2/versions/loader/";

#[derive(Deserialize)]
struct FabricVersionLoader {
    version: String,
}

#[derive(Deserialize)]
struct FabricVersionMeta {
    loader: FabricVersionLoader,
}

pub struct FabricVersionsMeta {
    versions: Vec<FabricVersionMeta>,
}

impl FabricVersionsMeta {
    pub async fn fetch(game_version: &str) -> BoxResult<Self> {
        let fabric_manifest_url = format!("{}{}", FABRIC_META_BASE_URL, game_version);
        let client = Client::new();
        let response = client
            .get(&fabric_manifest_url)
            .send()
            .await?
            .error_for_status()?;
        let fabric_versions: Vec<FabricVersionMeta> = response.json().await?;
        Ok(Self {
            versions: fabric_versions,
        })
    }

    pub fn get_versions(&self) -> Vec<&str> {
        self.versions
            .iter()
            .map(|version| version.loader.version.as_str())
            .collect()
    }

    pub fn get_latest_version(&self) -> Option<&str> {
        self.get_versions().first().map(|version| *version)
    }
}

async fn download_fabric_metadata(
    minecraft_version: &str,
    loader_version: &str,
    output_dir: &Path,
) -> BoxResult<VersionMetadata> {
    let fabric_metadata_url = format!(
        "{}{}/{}/profile/json",
        FABRIC_META_BASE_URL, minecraft_version, loader_version
    );
    let version_metadata = VersionMetadata::fetch(&fabric_metadata_url).await?;
    let versions_dir = get_versions_dir(&output_dir);
    version_metadata.save(&versions_dir).await?;
    Ok(version_metadata)
}

pub struct FabricGenerator {
    version_name: String,
    vanilla_version_info: VersionInfo,
    loader_version: Option<String>,
}

impl FabricGenerator {
    pub fn new(
        version_name: String,
        vanilla_version_info: VersionInfo,
        loader_version: Option<String>,
    ) -> Self {
        Self {
            version_name,
            vanilla_version_info,
            loader_version,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum FabricGeneratorError {
    #[error("No Fabric versions found for game version {0}")]
    NoVersionsFound(String),
}

#[async_trait]
impl VersionGenerator for FabricGenerator {
    async fn generate(&self, work_dir: &Path) -> BoxResult<GeneratorResult> {
        let minecraft_version = self.vanilla_version_info.id.clone();

        info!(
            "Generating Fabric instance \"{}\", minecraft version {}",
            self.version_name, minecraft_version
        );

        info!("Downloading vanilla version metadata");
        let vanilla_metadata = VersionMetadata::read_or_download(
            &self.vanilla_version_info.get_parent_metadata_info(),
            &get_versions_dir(work_dir),
        )
        .await?;

        let fabric_version = match &self.loader_version {
            Some(loader_version) => loader_version.clone(),
            None => {
                let meta = FabricVersionsMeta::fetch(&minecraft_version).await?;
                let version =
                    meta.get_latest_version()
                        .ok_or(FabricGeneratorError::NoVersionsFound(
                            minecraft_version.to_string(),
                        ))?;
                info!(
                    "Loader version not specified, using latest version: {}",
                    version
                );
                version.to_string()
            }
        };

        info!("Downloading Fabric version metadata");
        let fabric_metadata =
            download_fabric_metadata(&minecraft_version, &fabric_version, &work_dir).await?;

        info!("Fabric version \"{}\" generated", self.version_name);

        Ok(GeneratorResult {
            metadata: vec![vanilla_metadata, fabric_metadata],
            extra_libs_paths: vec![],
        })
    }
}
