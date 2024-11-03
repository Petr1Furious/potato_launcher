use std::path::{Path, PathBuf};

use crate::{utils::BoxResult, version::version_metadata::VersionMetadata};
use async_trait::async_trait;

pub struct GeneratorResult {
    // ordered from parent to child
    pub metadata: Vec<VersionMetadata>,

    pub extra_libs_paths: Vec<PathBuf>,
}

#[async_trait]
pub trait VersionGenerator {
    async fn generate(&self, work_dir: &Path) -> BoxResult<GeneratorResult>;
}
