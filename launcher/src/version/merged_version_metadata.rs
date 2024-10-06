use std::{error::Error, path::Path};

use shared::{
    files::{self, CheckDownloadEntry},
    progress,
    version::{
        version_manifest::VersionInfo,
        version_metadata::{
            get_version_metadata_path, read_version_metadata, Arguments, AssetIndex, Downloads,
            JavaVersion, Library, VersionMetadata,
        },
    },
};

#[derive(thiserror::Error, Debug)]
pub enum VersionMetadataError {
    #[error("Bad arguments")]
    BadArgumentsError,
}

pub struct MergedVersionMetadata {
    pub arguments: Arguments,
    pub asset_index: AssetIndex,
    pub id: String,
    pub java_version: JavaVersion,
    libraries: Vec<Library>,
    pub main_class: String,
    pub downloads: Option<Downloads>,
    pub hierarchy_ids: Vec<String>,
}

impl MergedVersionMetadata {
    fn from_version_metadata(
        version_metadata: VersionMetadata,
    ) -> Result<MergedVersionMetadata, Box<dyn Error + Send + Sync>> {
        Ok(MergedVersionMetadata {
            arguments: version_metadata.get_arguments()?,
            asset_index: version_metadata
                .asset_index
                .ok_or(Box::new(VersionMetadataError::BadArgumentsError))?,
            id: version_metadata.id.clone(),
            java_version: version_metadata
                .java_version
                .ok_or(Box::new(VersionMetadataError::BadArgumentsError))?,
            libraries: version_metadata.libraries,
            main_class: version_metadata.main_class,
            downloads: version_metadata.downloads,
            hierarchy_ids: vec![version_metadata.id],
        })
    }

    pub fn get_libraries(&self) -> Vec<&Library> {
        self.libraries.iter().collect()
    }
}

fn merge_two_metadata(
    child_metadata: &mut MergedVersionMetadata,
    parent_metadata: VersionMetadata,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(arguments) = parent_metadata.arguments {
        child_metadata.arguments.game.extend(arguments.game);
        child_metadata.arguments.jvm.extend(arguments.jvm);
    }
    child_metadata.libraries.extend(parent_metadata.libraries);

    if child_metadata.downloads.is_none() && parent_metadata.downloads.is_some() {
        child_metadata.downloads = parent_metadata.downloads;
    }

    child_metadata.hierarchy_ids.push(parent_metadata.id);

    Ok(())
}

pub async fn read_local_merged_version_metadata(
    version_id: &str,
    versions_dir: &Path,
) -> Result<MergedVersionMetadata, Box<dyn Error + Send + Sync>> {
    let mut metadata = read_version_metadata(versions_dir, version_id).await?;
    let mut inherits_from = metadata.inherits_from.clone();
    let mut merged_metadata = MergedVersionMetadata::from_version_metadata(metadata)?;
    while let Some(parent_id) = &inherits_from {
        metadata = read_version_metadata(versions_dir, parent_id).await?;
        inherits_from = metadata.inherits_from.clone();
        merge_two_metadata(&mut merged_metadata, metadata)?;
    }

    Ok(merged_metadata)
}

pub async fn get_merged_version_metadata(
    version_info: &VersionInfo,
    versions_dir: &Path,
) -> Result<MergedVersionMetadata, Box<dyn Error + Send + Sync>> {
    let metadata_info = version_info.get_metadata_info();

    let check_entries: Vec<CheckDownloadEntry> = metadata_info
        .iter()
        .map(|metadata| CheckDownloadEntry {
            url: metadata.url.clone(),
            remote_sha1: Some(metadata.sha1.clone()),
            path: get_version_metadata_path(versions_dir, &metadata.id),
        })
        .collect();

    let download_entries =
        files::get_download_entries(check_entries, progress::no_progress_bar()).await?;
    files::download_files(download_entries, progress::no_progress_bar()).await?;

    read_local_merged_version_metadata(&version_info.id, versions_dir).await
}
