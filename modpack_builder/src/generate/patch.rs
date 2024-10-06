use std::path::Path;

use shared::{
    files::hash_file,
    paths::{get_asset_index_path, get_client_jar_path, get_libraries_dir},
    version::version_metadata::{Download, LibraryDownloads, VersionMetadata},
};

use crate::utils::{get_assets_dir, get_url_from_path};

pub async fn replace_download_urls(
    version_name: &str,
    version_metadata: &mut VersionMetadata,
    download_server_base: &str,
    data_dir: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(downloads) = &mut version_metadata.downloads {
        if let Some(download) = &mut downloads.client {
            let client_path = get_client_jar_path(data_dir, &version_metadata.id);
            download.url = get_url_from_path(&client_path, data_dir, download_server_base)?;
        }
    }

    if let Some(asset_index) = &mut version_metadata.asset_index {
        let asset_index_path = get_asset_index_path(&get_assets_dir(data_dir), &asset_index.id);
        asset_index.url = get_url_from_path(&asset_index_path, data_dir, download_server_base)?;
    }

    for library in &mut version_metadata.libraries {
        if let Some(library_path) = library.get_path(&get_libraries_dir(data_dir, version_name)) {
            if let Some(downloads) = &mut library.downloads {
                if let Some(artifact) = &mut downloads.artifact {
                    artifact.url =
                        get_url_from_path(&library_path, data_dir, download_server_base)?;
                }
            } else if library.url.is_some() {
                let sha1 = if let Some(sha1) = &library.sha1 {
                    sha1.clone()
                } else {
                    hash_file(&library_path).await?
                };
                library.url = None;
                library.sha1 = None;
                library.downloads = Some(LibraryDownloads {
                    artifact: Some(Download {
                        url: get_url_from_path(&library_path, data_dir, download_server_base)?,
                        sha1,
                    }),
                    classifiers: None,
                });
            }
        }
    }

    for library in &mut version_metadata.libraries {
        let mut new_natives_urls = vec![];
        if let Some(downloads) = &library.downloads {
            if let Some(natives) = &downloads.classifiers {
                for (natives_name, download) in natives.clone() {
                    let libraries_dir = get_libraries_dir(data_dir, version_name);
                    let natives_path =
                        library.get_natives_path(&natives_name, &download, &libraries_dir);
                    new_natives_urls.push(get_url_from_path(
                        &natives_path,
                        data_dir,
                        download_server_base,
                    )?);
                }
            }
        }

        if !new_natives_urls.is_empty() {
            if let Some(downloads) = &mut library.downloads {
                if let Some(natives) = &mut downloads.classifiers {
                    for (download, new_url) in natives.values_mut().zip(new_natives_urls) {
                        download.url = new_url;
                    }
                }
            }
        }
    }

    Ok(())
}
