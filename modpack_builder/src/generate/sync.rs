use std::{path::Path, sync::Arc};

use log::{debug, info};
use shared::{
    files::{download_files, get_download_entries, CheckDownloadEntry},
    paths::{get_client_jar_path, get_libraries_dir},
    progress::ProgressBar,
    version::{asset_metadata::AssetsMetadata, version_metadata::VersionMetadata},
};

use crate::{progress::TerminalProgressBar, utils::get_assets_dir};

pub fn get_libraries_check_downloads(
    version_metadata: &VersionMetadata,
    libraries_dir: &Path,
) -> Vec<CheckDownloadEntry> {
    let mut entries = vec![];
    for library in &version_metadata.libraries {
        entries.extend(library.get_all_check_download_entries(libraries_dir));
    }
    debug!("Library check entries: {:?}", entries);
    entries
}

fn get_client_download_entry(
    version_metadata: &VersionMetadata,
    data_dir: &Path,
) -> Option<CheckDownloadEntry> {
    let client_download = version_metadata.downloads.as_ref()?.client.as_ref()?;

    Some(CheckDownloadEntry {
        url: client_download.url.clone(),
        remote_sha1: Some(client_download.sha1.clone()),
        path: get_client_jar_path(data_dir, &version_metadata.id),
    })
}

const RESOURCES_URL_BASE: &str = "https://resources.download.minecraft.net";

pub async fn sync_version(
    version_metadata: &VersionMetadata,
    version_name: &str,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let libraries_dir = get_libraries_dir(output_dir, version_name);
    let mut check_entries = get_libraries_check_downloads(version_metadata, &libraries_dir);
    info!("Got {} libraries to check", check_entries.len());

    if let Some(asset_index) = &version_metadata.asset_index {
        let assets_dir = get_assets_dir(output_dir);
        let assets_metadata = AssetsMetadata::read_or_fetch(asset_index, &assets_dir).await?;
        let asset_check_entries =
            assets_metadata.get_check_downloads(&assets_dir, RESOURCES_URL_BASE)?;
        assets_metadata
            .save_to_file(&asset_index.id, &assets_dir)
            .await?;

        let mut already_have = 0;
        for entry in &asset_check_entries {
            if entry.path.exists() {
                already_have += 1;
            }
        }
        info!(
            "Already have {}/{} assets",
            already_have,
            asset_check_entries.len()
        );

        check_entries.extend(asset_check_entries);
    }

    if let Some(client_entry) = get_client_download_entry(version_metadata, output_dir) {
        info!("Got client.jar to check");
        check_entries.push(client_entry);
    }

    let progress_bar = Arc::new(TerminalProgressBar::new());

    progress_bar.set_message("Checking files...");
    let download_entries = get_download_entries(check_entries, progress_bar.clone()).await?;

    progress_bar.reset();
    progress_bar.set_message("Downloading files...");
    download_files(download_entries, progress_bar).await?;

    Ok(())
}
