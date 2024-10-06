use std::{
    error::Error,
    path::{Path, PathBuf},
};

use fs_extra::{dir, file};
use log::{error, info};
use shared::{
    files::{self, get_files_in_dir},
    paths::{get_minecraft_dir, get_versions_extra_dir},
    progress,
    version::{
        extra_version_metadata::{save_extra_version_metadata, ExtraVersionMetadata, Object},
        version_metadata::Download,
    },
};

use crate::utils::get_url_from_path;

#[derive(thiserror::Error, Debug)]
pub enum ExtraMetadataGeneratorError {
    #[error("Path does not exist: {0}")]
    PathDoesNotExist(String),
}

fn sync_paths(from: &Path, to: &Path) -> Result<(), Box<dyn Error + Send + Sync>> {
    if !from.exists() {
        return Err(Box::new(ExtraMetadataGeneratorError::PathDoesNotExist(
            from.to_string_lossy().to_string(),
        )));
    }

    if from.is_file() && to.is_dir() {
        // scary
        std::fs::remove_dir_all(to)?;
    }
    if from.is_dir() && to.is_file() {
        std::fs::remove_file(to)?;
    }

    if from.is_file() {
        let mut options = file::CopyOptions::new();
        options.overwrite = true;
        options.skip_exist = true;

        file::copy(from, to, &options)?;
    } else {
        let mut options = dir::CopyOptions::new();
        options.overwrite = true;
        options.skip_exist = true;

        dir::copy(from, to.parent().unwrap(), &options)?;
    }

    Ok(())
}

async fn get_objects(
    path: &Path,
    data_dir: &Path,
    version_name: &str,
    download_server_base: &str,
) -> Result<Vec<Object>, Box<dyn Error + Send + Sync>> {
    let minecraft_dir = get_minecraft_dir(data_dir, version_name);

    let paths = if path.is_file() {
        vec![path.to_path_buf()]
    } else {
        get_files_in_dir(path)
    };

    let hashes = files::hash_files(paths.clone(), progress::no_progress_bar()).await?;

    let mut objects = vec![];
    for (path, hash) in paths.iter().zip(hashes.iter()) {
        let url = get_url_from_path(path, data_dir, download_server_base)?;
        objects.push(Object {
            path: path.strip_prefix(&minecraft_dir)?.to_string_lossy().to_string(),
            sha1: hash.clone(),
            url,
        });
    }

    Ok(objects)
}

async fn get_client_override(
    path: &Path,
    data_dir: &Path,
    download_server_base: &str,
) -> Result<Download, Box<dyn Error + Send + Sync>> {
    Ok(Download {
        url: get_url_from_path(path, data_dir, download_server_base)?,
        sha1: files::hash_file(path).await.unwrap(),
    })
}

pub struct ExtraMetadataGenerator {
    version_name: String,
    include: Vec<String>,
    include_no_overwrite: Vec<String>,
    include_from: Option<String>,
    resources_url_base: Option<String>,
    download_server_base: String,
    client_override_path: Option<PathBuf>,
}

impl ExtraMetadataGenerator {
    pub fn new(
        version_name: String,
        include: Vec<String>,
        include_no_overwrite: Vec<String>,
        include_from: Option<String>,
        resources_url_base: Option<String>,
        download_server_base: String,
        client_override_path: Option<PathBuf>,
    ) -> Self {
        Self {
            version_name,
            include,
            include_no_overwrite,
            include_from,
            resources_url_base,
            download_server_base,
            client_override_path,
        }
    }

    pub async fn generate(&self, output_dir: &Path) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!(
            "Generating extra metadata for modpack {}",
            self.version_name
        );

        if self.include_from.is_none() && self.resources_url_base.is_none() {
            return Ok(());
        }

        let client_override = if let Some(client_override_path) = &self.client_override_path {
            Some(
                get_client_override(
                    &client_override_path,
                    output_dir,
                    &self.download_server_base,
                )
                .await?,
            )
        } else {
            None
        };

        let mut extra_metadata = ExtraVersionMetadata {
            version_name: self.version_name.clone(),
            include: self.include.clone(),
            include_no_overwrite: self.include_no_overwrite.clone(),
            objects: Vec::new(),
            resources_url_base: self.resources_url_base.clone(),
            client_override,
        };

        if let Some(include_from) = &self.include_from {
            let copy_from = PathBuf::from(include_from);
            let copy_to = get_minecraft_dir(output_dir, &self.version_name);

            let mut objects = vec![];
            for include in self.include.iter().chain(self.include_no_overwrite.iter()) {
                let from = copy_from.join(include);
                let to = copy_to.join(include);

                info!(
                    "Copying {} from {} to {}",
                    include,
                    from.to_string_lossy(),
                    to.to_string_lossy()
                );
                sync_paths(&from, &to)?;

                objects.extend(get_objects(&to, output_dir, &self.version_name, &self.download_server_base).await?);
            }

            extra_metadata.objects = objects;
        }

        let versions_extra_dir = get_versions_extra_dir(output_dir);
        save_extra_version_metadata(&versions_extra_dir, &self.version_name, &extra_metadata)
            .await?;

        info!("Extra metadata for modpack {} generated", self.version_name);

        Ok(())
    }
}
