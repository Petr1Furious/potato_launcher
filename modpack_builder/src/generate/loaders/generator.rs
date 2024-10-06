use std::{error::Error, path::Path};

use async_trait::async_trait;

#[async_trait]
pub trait VersionGenerator {
    async fn generate(
        &self,
        output_dir: &Path,
        work_dir: &Path,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;
}
