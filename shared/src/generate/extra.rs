use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    files,
    paths::{get_libraries_dir, get_rel_instance_dir, get_versions_extra_dir},
    progress::{self, NoProgressBar, ProgressBar as _},
    utils::{url_from_path, url_from_rel_path, BoxResult},
    version::{
        extra_version_metadata::{AuthData, ExtraVersionMetadata, Object},
        version_metadata::Library,
    },
};
use log::info;

async fn get_objects(
    copy_from: &Path,
    from: &Path,
    download_server_base: &str,
    version_name: &str,
) -> BoxResult<Vec<Object>> {
    let files_in_dir = files::get_files_in_dir(from)?;

    let rel_paths = files_in_dir
        .iter()
        .map(|p| p.strip_prefix(copy_from))
        .collect::<Result<Vec<_>, _>>()?;
    let hashes = files::hash_files(files_in_dir.clone(), progress::no_progress_bar()).await?;

    let mut objects = vec![];
    for (rel_path, hash) in rel_paths.iter().zip(hashes.iter()) {
        let url = url_from_rel_path(
            &get_rel_instance_dir(version_name).join(rel_path),
            download_server_base,
        )?;
        objects.push(Object {
            path: rel_path.to_string_lossy().to_string(),
            sha1: hash.clone(),
            url,
        });
    }

    Ok(objects)
}

const AUTHLIB_INJECTOR_URL: &str = "https://github.com/yushijinhun/authlib-injector/releases/download/v1.2.5/authlib-injector-1.2.5.jar";
const AUTHLIB_INJECTOR_FILENAME: &str = "authlib-injector.jar";

async fn download_authlib_injector(
    work_dir: &Path,
    download_server_base: &str,
) -> BoxResult<Object> {
    let authlib_injector_path = work_dir.join(AUTHLIB_INJECTOR_FILENAME);
    if !authlib_injector_path.exists() {
        info!("Downloading authlib-injector");
        let client = reqwest::Client::new();
        files::download_file(&client, AUTHLIB_INJECTOR_URL, &authlib_injector_path).await?;
    }

    info!("Adding authlib-injector to extra metadata");

    let hash = files::hash_file(&authlib_injector_path).await?;
    let url = url_from_rel_path(
        &PathBuf::from(AUTHLIB_INJECTOR_FILENAME),
        download_server_base,
    )?;

    Ok(Object {
        path: AUTHLIB_INJECTOR_FILENAME.to_string(),
        sha1: hash,
        url,
    })
}

#[derive(thiserror::Error, Debug)]
pub enum ExtraForgeLibsError {
    #[error("Bad library name: {0}")]
    BadLibraryName(String),
}

async fn get_extra_forge_libs(
    extra_forge_libs_paths: &Vec<PathBuf>,
    data_dir: &Path,
    download_server_base: &str,
) -> BoxResult<Vec<Library>> {
    let libraries_dir = get_libraries_dir(data_dir);

    let progress_bar = Arc::new(NoProgressBar);
    progress_bar.set_message("Hashing extra forge libraries");
    let hashes = files::hash_files::<&str>(extra_forge_libs_paths.to_vec(), progress_bar).await?;

    let libraries = extra_forge_libs_paths
        .iter()
        .zip(hashes.iter())
        .filter(|(path, _)| path.is_file() && path.extension().map_or(false, |ext| ext == "jar"))
        .map(|(path, hash)| {
            let url = url_from_path(path, data_dir, download_server_base)?;

            let parts = path
                .strip_prefix(&libraries_dir)?
                .components()
                .map(|x| x.as_os_str().to_string_lossy())
                .collect::<Vec<_>>();
            let version = parts[parts.len() - 2].to_string();
            let name = parts[parts.len() - 3].to_string();
            let group = parts
                .iter()
                .take(parts.len() - 3)
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(".");

            let filename = path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .strip_suffix(".jar")
                .unwrap()
                .to_string();
            let filename_without_suffix = format!("{}-{}", name, version);
            let suffix = filename
                .strip_prefix(&filename_without_suffix)
                .ok_or(ExtraForgeLibsError::BadLibraryName(filename.clone()))?;
            let suffix = suffix.replace("-", ":");

            let name = format!("{}:{}:{}{}", group, name, version, suffix);

            Ok(Library::from_download(name, url, hash.clone()))
        })
        .collect::<BoxResult<_>>()?;

    Ok(libraries)
}

pub struct ExtraMetadataGenerator {
    version_name: String,
    include: Vec<String>,
    include_no_overwrite: Vec<String>,
    include_from: Option<String>,
    resources_url_base: Option<String>,
    download_server_base: String,
    extra_forge_libs_paths: Vec<PathBuf>,
    auth_data: AuthData,
}

pub struct GeneratorResult {
    pub paths_to_copy: Vec<PathBuf>,

    // relative include path -> absolute source path
    pub include_mapping: HashMap<String, PathBuf>,

    pub extra_metadata: ExtraVersionMetadata,
}

impl ExtraMetadataGenerator {
    pub fn new(
        version_name: String,
        include: Vec<String>,
        include_no_overwrite: Vec<String>,
        include_from: Option<String>,
        resources_url_base: Option<String>,
        download_server_base: String,
        extra_forge_libs_paths: Vec<PathBuf>,
        auth_data: AuthData,
    ) -> Self {
        Self {
            version_name,
            include,
            include_no_overwrite,
            include_from,
            resources_url_base,
            download_server_base,
            extra_forge_libs_paths,
            auth_data,
        }
    }

    pub async fn generate(&self, work_dir: &Path) -> BoxResult<GeneratorResult> {
        info!(
            "Generating extra metadata for instance {}",
            self.version_name
        );

        let extra_forge_libs = get_extra_forge_libs(
            &self.extra_forge_libs_paths,
            work_dir,
            &self.download_server_base,
        )
        .await?;

        let mut extra_metadata = ExtraVersionMetadata {
            include: self.include.clone(),
            include_no_overwrite: self.include_no_overwrite.clone(),
            objects: vec![],
            resources_url_base: self.resources_url_base.clone(),
            auth_provider: self.auth_data.clone(),
            extra_forge_libs,
            authlib_injector: None,
        };
        let mut paths_to_copy = vec![];
        let mut include_mapping = HashMap::new();

        if let Some(include_from) = &self.include_from {
            let mut objects = vec![];
            let copy_from = PathBuf::from(include_from);

            for include in self.include.iter().chain(self.include_no_overwrite.iter()) {
                let from = copy_from.join(include);

                objects.extend(
                    get_objects(
                        &copy_from,
                        &from,
                        &self.download_server_base,
                        &self.version_name,
                    )
                    .await?,
                );
                include_mapping.insert(include.clone(), from);
            }

            extra_metadata.objects = objects;
        }

        match self.auth_data {
            AuthData::None => {}
            _ => {
                extra_metadata.authlib_injector =
                    Some(download_authlib_injector(work_dir, &self.download_server_base).await?);

                paths_to_copy.push(work_dir.join(AUTHLIB_INJECTOR_FILENAME));
            }
        }

        let versions_extra_dir = get_versions_extra_dir(work_dir);
        extra_metadata
            .save(&self.version_name, &versions_extra_dir)
            .await?;

        info!(
            "Extra metadata for instance {} generated",
            self.version_name
        );

        Ok(GeneratorResult {
            paths_to_copy,
            include_mapping,
            extra_metadata,
        })
    }
}
