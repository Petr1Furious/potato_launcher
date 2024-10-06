use std::path::Path;

use shared::{
    files::hash_file,
    paths::{get_versions_dir, get_versions_extra_dir},
    version::{
        extra_version_metadata::get_extra_version_metadata_path,
        version_manifest::{MetadataInfo, VersionInfo},
        version_metadata::{get_version_metadata_path, read_version_metadata},
    },
};

use crate::utils::get_url_from_path;

pub async fn get_version_info(
    output_dir: &Path,
    version_id: &str,
    version_name: &str,
    download_server_base: &str,
) -> Result<VersionInfo, Box<dyn std::error::Error + Send + Sync>> {
    let versions_dir = get_versions_dir(output_dir);

    let version_path = get_version_metadata_path(&versions_dir, version_id);

    let mut inherits_from = vec![];
    let mut id = version_id.to_string();
    loop {
        let version_metadata = read_version_metadata(&versions_dir, &id).await?;
        if let Some(new_id) = version_metadata.inherits_from {
            id = new_id;
            let version_path = get_version_metadata_path(&versions_dir, &id);
            inherits_from.push(MetadataInfo {
                id: id.clone(),
                url: get_url_from_path(&version_path, output_dir, download_server_base)?,
                sha1: hash_file(&version_path).await?,
            });
        } else {
            break;
        }
    }

    let versions_extra_dir = get_versions_extra_dir(output_dir);
    let extra_metadata_path = get_extra_version_metadata_path(&versions_extra_dir, version_name);

    let mut extra_metadata_url = None;
    let mut extra_metadata_sha1 = None;
    if extra_metadata_path.exists() {
        extra_metadata_url = Some(get_url_from_path(
            &extra_metadata_path,
            output_dir,
            download_server_base,
        )?);
        extra_metadata_sha1 = Some(hash_file(&extra_metadata_path).await?);
    }

    Ok(VersionInfo {
        id: version_id.to_string(),
        url: get_url_from_path(&version_path, output_dir, download_server_base)?,
        sha1: hash_file(&version_path).await?,
        name: Some(version_name.to_string()),
        inherits_from,
        extra_metadata_url,
        extra_metadata_sha1,
    })
}
