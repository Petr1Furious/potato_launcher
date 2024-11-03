use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use log::{debug, info};
use shared::paths::{get_instance_dir, get_libraries_dir, get_natives_dir};
use shared::utils::BoxResult;
use shared::version::asset_metadata::AssetsMetadata;
use std::fs;
use zip::ZipArchive;

use shared::files::{self, CheckEntry};
use shared::progress::{self, ProgressBar};
use shared::version::extra_version_metadata::ExtraVersionMetadata;
use shared::version::version_metadata;

use crate::lang::LangMessage;

use super::complete_version_metadata::CompleteVersionMetadata;
use super::rules;

#[derive(thiserror::Error, Debug)]
pub enum VersionMetadataError {
    #[error("Library {0} has neither SHA1 hash nor SHA1 URL")]
    NoSha1(String),
}

fn get_objects_entries(
    extra_version_metadata: &ExtraVersionMetadata,
    force_overwrite: bool,
    instance_dir: &Path,
) -> BoxResult<Vec<CheckEntry>> {
    let objects = &extra_version_metadata.objects;
    let include = &extra_version_metadata.include;
    let include_no_overwrite = &extra_version_metadata.include_no_overwrite;

    let get_instance_files = |x| files::get_files_in_dir(&instance_dir.join(x)).ok();
    let no_overwrite_iter = include_no_overwrite
        .iter()
        .filter_map(get_instance_files)
        .flatten();
    let mut to_overwrite: HashSet<PathBuf> = include
        .iter()
        .filter_map(get_instance_files)
        .flatten()
        .collect();
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
        objects.iter().map(|x| instance_dir.join(&x.path)).collect();
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
        let object_path = instance_dir.join(&object.path);

        if no_overwrite.contains(&object_path) {
            continue;
        }
        download_entries.push(CheckEntry {
            url: object.url.clone(),
            remote_sha1: Some(object.sha1.clone()),
            path: object_path,
        });
    }

    Ok(download_entries)
}

async fn get_libraries_entries(
    libraries: &Vec<version_metadata::Library>,
    libraries_dir: &Path,
) -> BoxResult<Vec<CheckEntry>> {
    let mut sha1_urls = HashMap::<PathBuf, String>::new();
    let mut check_download_entries: Vec<CheckEntry> = Vec::new();

    for library in libraries {
        for entry in rules::get_check_download_entries(library, libraries_dir) {
            if entry.remote_sha1.is_some() || !entry.path.exists() {
                if entry.url == "" {
                    info!("Skipping library with no URL: {:?}", entry.path);
                    continue;
                }
                check_download_entries.push(entry);
            } else {
                match library.get_sha1_url() {
                    Some(sha1_url) => {
                        sha1_urls.insert(entry.path.clone(), sha1_url);
                        check_download_entries.push(CheckEntry {
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
                    Ok(CheckEntry {
                        remote_sha1: Some(sha1.clone()),
                        ..entry
                    })
                } else if !entry.path.exists() {
                    Ok(entry)
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
) -> BoxResult<()> {
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

fn extract_files(src: &Path, dest: &Path, exclude: Option<HashSet<String>>) -> BoxResult<()> {
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

fn get_authlib_injector_entry(
    version_metadata: &CompleteVersionMetadata,
    launcher_dir: &Path,
) -> Option<CheckEntry> {
    if let Some(extra) = version_metadata.get_extra() {
        if let Some(authlib_injector) = &extra.authlib_injector {
            return Some(CheckEntry {
                url: authlib_injector.url.clone(),
                remote_sha1: Some(authlib_injector.sha1.clone()),
                path: launcher_dir.join(&authlib_injector.path),
            });
        }
    }

    None
}

pub async fn sync_instance(
    version_metadata: &CompleteVersionMetadata,
    force_overwrite: bool,
    launcher_dir: &Path,
    assets_dir: &Path,
    progress_bar: Arc<dyn ProgressBar<LangMessage> + Send + Sync>,
) -> BoxResult<()> {
    let version_name = version_metadata.get_name();

    let libraries_dir = get_libraries_dir(launcher_dir);
    let natives_dir = get_natives_dir(launcher_dir, version_metadata.get_parent_id());
    let instance_dir = get_instance_dir(launcher_dir, &version_name);

    let mut check_entries = vec![];

    check_entries.push(version_metadata.get_client_check_entry(launcher_dir)?);

    let mut libraries = version_metadata.get_libraries_with_overrides();
    libraries.extend(version_metadata.get_extra_forge_libs());
    check_entries.extend(get_libraries_entries(&libraries, &libraries_dir).await?);

    if let Some(extra) = version_metadata.get_extra() {
        check_entries.extend(get_objects_entries(extra, force_overwrite, &instance_dir)?);
    }

    if let Some(authlib_injector) = get_authlib_injector_entry(version_metadata, launcher_dir) {
        check_entries.push(authlib_injector);
    }

    let asset_index = version_metadata.get_asset_index()?;
    let asset_metadata = AssetsMetadata::read_or_download(asset_index, assets_dir).await?;

    check_entries.extend(
        asset_metadata.get_check_entries(assets_dir, version_metadata.get_resources_url_base())?,
    );

    info!("Got {} check download entries", check_entries.len());
    progress_bar.set_message(LangMessage::CheckingFiles);
    let download_entries = files::get_download_entries(check_entries, progress_bar.clone()).await?;

    info!("Got {} download entries", download_entries.len());

    let paths = download_entries
        .iter()
        .map(|x| x.path.clone())
        .collect::<Vec<_>>();
    debug!("Paths to download: {:?}", paths);

    progress_bar.set_message(LangMessage::DownloadingFiles);
    files::download_files(download_entries, progress_bar).await?;

    extract_natives(&libraries, &libraries_dir, &natives_dir)?;

    Ok(())
}
