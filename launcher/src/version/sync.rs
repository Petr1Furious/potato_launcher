use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use shared::paths::{get_client_jar_path, get_libraries_dir, get_minecraft_dir, get_natives_dir};
use shared::version::asset_metadata::AssetsMetadata;
use std::fs;
use zip::ZipArchive;

use shared::files::{self, CheckDownloadEntry};
use shared::progress::{self, ProgressBar};
use shared::version::extra_version_metadata::ExtraVersionMetadata;
use shared::version::version_metadata;

use crate::lang::LangMessage;

use super::complete_version_metadata::CompleteVersionMetadata;
use super::overrides::with_overrides;
use super::rules;

#[derive(thiserror::Error, Debug)]
pub enum VersionMetadataError {
    #[error("Library {0} has neither SHA1 hash nor SHA1 URL")]
    NoSha1(String),
    #[error("Missing client download")]
    MissingClientDownload,
}

async fn get_objects_downloads(
    extra_version_metadata: &ExtraVersionMetadata,
    force_overwrite: bool,
    modpack_dir: &Path,
) -> Result<Vec<CheckDownloadEntry>, Box<dyn std::error::Error + Send + Sync>> {
    let objects = &extra_version_metadata.objects;
    let include = &extra_version_metadata.include;
    let include_no_overwrite = &extra_version_metadata.include_no_overwrite;

    let get_modpack_files = |x| files::get_files_in_dir(&modpack_dir.join(x));
    let no_overwrite_iter = include_no_overwrite.iter().map(get_modpack_files).flatten();
    let mut to_overwrite: HashSet<PathBuf> =
        include.iter().map(get_modpack_files).flatten().collect();
    let mut no_overwrite = HashSet::new();
    if !force_overwrite {
        no_overwrite.extend(no_overwrite_iter);
    } else {
        to_overwrite.extend(no_overwrite_iter);
    }

    // Remove files that are in both no_overwrite and overwrite
    // e.g. config folder is in no_overwrite but config/<filename>.json is in overwrite
    no_overwrite.retain(|x| !to_overwrite.contains(x));

    // delete extra to_overwrite files
    let objects_hashset: HashSet<PathBuf> =
        objects.iter().map(|x| modpack_dir.join(&x.path)).collect();
    let _ = to_overwrite
        .iter()
        .map(|x| {
            if !objects_hashset.contains(x) {
                fs::remove_file(x).unwrap();
            }
        })
        .collect::<Vec<()>>();

    let mut download_entries = vec![];
    for object in objects.iter() {
        let object_path = modpack_dir.join(&object.path);

        if no_overwrite.contains(&object_path) {
            continue;
        }
        download_entries.push(CheckDownloadEntry {
            url: object.url.clone(),
            remote_sha1: Some(object.sha1.clone()),
            path: object_path,
        });
    }

    Ok(download_entries)
}

async fn get_libraries_downloads(
    libraries: &Vec<version_metadata::Library>,
    libraries_dir: &Path,
) -> Result<Vec<CheckDownloadEntry>, Box<dyn std::error::Error + Send + Sync>> {
    let mut sha1_urls = HashMap::<PathBuf, String>::new();
    let mut check_download_entries: Vec<CheckDownloadEntry> = Vec::new();

    for library in libraries {
        for entry in rules::get_check_download_entries(library, libraries_dir) {
            if entry.remote_sha1.is_some() || !entry.path.exists() {
                check_download_entries.push(entry);
            } else {
                match library.get_sha1_url() {
                    Some(sha1_url) => {
                        sha1_urls.insert(entry.path.clone(), sha1_url);
                        check_download_entries.push(CheckDownloadEntry {
                            remote_sha1: None,
                            ..entry
                        });
                    }
                    None => {
                        return Err(Box::new(VersionMetadataError::NoSha1(
                            entry.path.to_str().unwrap_or("no path").to_string(),
                        )));
                    }
                }
            }
        }
    }

    let missing_hash_urls: Vec<String> = sha1_urls.values().cloned().collect();
    let remote_hashes = files::fetch_files(missing_hash_urls, progress::no_progress_bar()).await?;
    let remote_hashes: Vec<String> = remote_hashes
        .into_iter()
        .map(|x| String::from_utf8(x))
        .collect::<Result<_, _>>()?;
    let missing_hashes: HashMap<PathBuf, String> =
        sha1_urls.into_keys().zip(remote_hashes).collect();

    let check_download_entries: Vec<_> = check_download_entries
        .into_iter()
        .map(|entry| {
            if entry.remote_sha1.is_none() {
                let sha1 = missing_hashes.get(&entry.path);
                if let Some(sha1) = sha1 {
                    Ok(CheckDownloadEntry {
                        remote_sha1: Some(sha1.clone()),
                        ..entry
                    })
                } else {
                    Err(Box::new(VersionMetadataError::NoSha1(
                        entry.path.to_str().unwrap_or("no path").to_string(),
                    )))
                }
            } else {
                Ok(entry)
            }
        })
        .collect::<Result<_, _>>()?;

    Ok(check_download_entries)
}

fn extract_natives(
    libraries: &Vec<version_metadata::Library>,
    libraries_dir: &Path,
    natives_dir: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for library in libraries {
        if let Some(natives_path) = rules::get_natives_path(library, libraries_dir) {
            let exclude = library.get_extract().map(|x| {
                x.exclude
                    .clone()
                    .unwrap_or_default()
                    .into_iter()
                    .collect::<HashSet<_>>()
            });
            extract_files(&natives_path, &natives_dir, exclude)?;
        }
    }

    Ok(())
}

fn extract_files(
    src: &Path,
    dest: &Path,
    exclude: Option<HashSet<String>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let exclude = exclude.unwrap_or_default();

    let file = fs::File::open(src)?;
    let mut zip = ZipArchive::new(file)?;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        if let Some(file_path) = entry.enclosed_name() {
            if let Some(directory) = file_path.components().next() {
                let directory = directory.as_os_str().to_str().unwrap_or_default();
                if exclude.contains(directory)
                    || exclude.contains(format!("{}/", directory).as_str())
                {
                    continue;
                }
            }

            let output_path = dest.join(file_path);
            if entry.is_file() {
                if let Some(parent) = output_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut outfile = fs::File::create(&output_path)?;
                std::io::copy(&mut entry, &mut outfile)?;
            } else if entry.is_dir() {
                fs::create_dir_all(&output_path)?;
            }
        }
    }

    Ok(())
}

fn get_client_download_entry(
    version_metadata: &CompleteVersionMetadata,
    launcher_dir: &Path,
) -> Result<CheckDownloadEntry, Box<dyn std::error::Error + Send + Sync>> {
    let is_overridden;
    if let Some(extra) = &version_metadata.extra {
        is_overridden = extra.client_override.is_some();
    } else {
        is_overridden = false;
    }

    let client_download = if is_overridden {
        version_metadata
            .extra
            .as_ref()
            .unwrap()
            .client_override
            .as_ref()
            .unwrap()
    } else {
        version_metadata
            .base
            .downloads
            .as_ref()
            .ok_or(VersionMetadataError::MissingClientDownload)?
            .client
            .as_ref()
            .ok_or(VersionMetadataError::MissingClientDownload)?
    };

    Ok(CheckDownloadEntry {
        url: client_download.url.clone(),
        remote_sha1: Some(client_download.sha1.clone()),
        path: get_client_jar_path(launcher_dir, &version_metadata.base.id),
    })
}

pub struct SyncData {
    pub launcher_dir: PathBuf,
    pub assets_dir: PathBuf,
    pub version_name: String,
}

pub async fn sync_modpack(
    version_metadata: &CompleteVersionMetadata,
    force_overwrite: bool,
    path_data: SyncData,
    progress_bar: Arc<dyn ProgressBar<LangMessage> + Send + Sync>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let SyncData {
        launcher_dir,
        assets_dir,
        version_name,
    } = path_data;

    let libraries_dir = get_libraries_dir(&launcher_dir, &version_name);
    let natives_dir = get_natives_dir(&launcher_dir, &version_name);
    let modpack_dir = get_minecraft_dir(&launcher_dir, &version_name);

    let mut check_download_entries = vec![];

    check_download_entries.push(get_client_download_entry(version_metadata, &launcher_dir)?);

    let libraries = with_overrides(
        version_metadata.base.get_libraries(),
        &version_metadata.base.hierarchy_ids,
    );
    check_download_entries.extend(get_libraries_downloads(&libraries, &libraries_dir).await?);

    if let Some(extra) = &version_metadata.extra {
        check_download_entries
            .extend(get_objects_downloads(extra, force_overwrite, &modpack_dir).await?);
    }

    let asset_index = &version_metadata.base.asset_index;
    let asset_metadata = AssetsMetadata::read_or_fetch(asset_index, &assets_dir).await?;

    check_download_entries.extend(
        asset_metadata
            .get_check_downloads(&assets_dir, version_metadata.get_resources_url_base())?,
    );

    progress_bar.set_message(LangMessage::CheckingFiles);
    let download_entries =
        files::get_download_entries(check_download_entries, progress_bar.clone()).await?;

    let libraries_changed = download_entries
        .iter()
        .any(|entry| entry.path.starts_with(&libraries_dir));

    progress_bar.set_message(LangMessage::DownloadingFiles);
    files::download_files(download_entries, progress_bar).await?;

    asset_metadata
        .save_to_file(&asset_index.id, &assets_dir)
        .await?;

    if libraries_changed {
        extract_natives(&libraries, &libraries_dir, &natives_dir)?;
    }

    Ok(())
}
