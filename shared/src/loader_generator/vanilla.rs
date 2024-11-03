use std::path::Path;

use crate::{
    paths::get_versions_dir,
    utils::BoxResult,
    version::{version_manifest::VersionInfo, version_metadata::VersionMetadata},
};
use async_trait::async_trait;
use log::info;

use super::generator::{GeneratorResult, VersionGenerator};

pub struct VanillaGenerator {
    version_name: String,
    version_info: VersionInfo,
}

impl VanillaGenerator {
    pub fn new(version_name: String, version_info: VersionInfo) -> Self {
        Self {
            version_name,
            version_info,
        }
    }
}

#[async_trait]
impl VersionGenerator for VanillaGenerator {
    async fn generate(&self, work_dir: &Path) -> BoxResult<GeneratorResult> {
        info!(
            "Generating vanilla version \"{}\", minecraft version {}",
            self.version_name, self.version_info.id
        );

        info!("Downloading version metadata");
        let vanilla_metadata = VersionMetadata::read_or_download(
            &self.version_info.get_parent_metadata_info(),
            &get_versions_dir(work_dir),
        )
        .await?;

        info!("Vanilla version \"{}\" generated", self.version_name);

        Ok(GeneratorResult {
            metadata: vec![vanilla_metadata],
            extra_libs_paths: vec![],
        })
    }
}
