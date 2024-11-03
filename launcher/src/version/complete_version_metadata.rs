use std::path::Path;

use shared::{
    files::{self, CheckEntry},
    paths::{get_client_jar_path, get_versions_dir, get_versions_extra_dir},
    progress,
    utils::BoxResult,
    version::{
        extra_version_metadata::{AuthData, ExtraVersionMetadata},
        version_manifest::VersionInfo,
        version_metadata::{Arguments, AssetIndex, Library, VersionMetadata},
    },
};

use super::overrides::with_overrides;

pub struct CompleteVersionMetadata {
    version_name: String,
    // ordered from parent to child
    base: Vec<VersionMetadata>,
    extra: Option<ExtraVersionMetadata>,
}

const DEFAULT_RESOURCES_URL_BASE: &str = "https://resources.download.minecraft.net";

const AUTH_DATA_NONE: AuthData = AuthData::None;

#[derive(thiserror::Error, Debug)]
pub enum VersionMetadataError {
    #[error("Missing asset index")]
    MissingAssetIndex,
    #[error("Missing client download")]
    MissingClientDownload,
}

impl CompleteVersionMetadata {
    pub fn from_parts(
        version_name: String,
        base: Vec<VersionMetadata>,
        extra: Option<ExtraVersionMetadata>,
    ) -> Self {
        Self {
            version_name,
            base,
            extra,
        }
    }

    pub async fn read_local(version_info: &VersionInfo, data_dir: &Path) -> BoxResult<Self> {
        let versions_dir = get_versions_dir(data_dir);

        let mut base = vec![];
        let mut version_id = version_info.id.to_string();
        loop {
            let current_metadata = VersionMetadata::read_local(&versions_dir, &version_id).await?;
            let parent_id = current_metadata.inherits_from.clone();
            base.push(current_metadata);
            if let Some(id) = parent_id {
                version_id = id;
            } else {
                break;
            }
        }
        base = base.into_iter().rev().collect();

        let versions_extra_dir = get_versions_extra_dir(data_dir);
        let extra = ExtraVersionMetadata::read_local(version_info, &versions_extra_dir).await?;

        Ok(Self {
            version_name: version_info.get_name(),
            base,
            extra,
        })
    }

    pub async fn read_or_download(version_info: &VersionInfo, data_dir: &Path) -> BoxResult<Self> {
        let versions_dir = get_versions_dir(data_dir);
        let versions_extra_dir = get_versions_extra_dir(data_dir);

        let metadata_info = version_info.get_metadata_info();

        let mut check_entries: Vec<CheckEntry> = metadata_info
            .iter()
            .map(|metadata_info| VersionMetadata::get_check_entry(metadata_info, &versions_dir))
            .collect();

        if let Some(check_entry) =
            ExtraVersionMetadata::get_check_entry(version_info, &versions_extra_dir)
        {
            check_entries.push(check_entry);
        }

        let download_entries =
            files::get_download_entries(check_entries, progress::no_progress_bar()).await?;
        files::download_files(download_entries, progress::no_progress_bar()).await?;

        Self::read_local(version_info, data_dir).await
    }

    pub fn get_resources_url_base(&self) -> &str {
        if let Some(extra) = &self.extra {
            return extra
                .resources_url_base
                .as_ref()
                .map(|x| x.as_str())
                .unwrap_or(DEFAULT_RESOURCES_URL_BASE);
        } else {
            return DEFAULT_RESOURCES_URL_BASE;
        }
    }

    pub fn get_java_version(&self) -> String {
        return self.base[0]
            .java_version
            .as_ref()
            .map(|x| x.major_version.to_string())
            .unwrap_or("8".to_string());
    }

    pub fn get_name(&self) -> &str {
        &self.version_name
    }

    pub fn get_client_check_entry(&self, launcher_dir: &Path) -> BoxResult<CheckEntry> {
        if let Some(downloads) = self.base[0].downloads.as_ref() {
            if let Some(client) = downloads.client.as_ref() {
                return Ok(CheckEntry {
                    url: client.url.clone(),
                    remote_sha1: Some(client.sha1.clone()),
                    path: get_client_jar_path(launcher_dir, self.get_id()),
                });
            }
        }

        Err(VersionMetadataError::MissingClientDownload.into())
    }

    pub fn get_auth_data(&self) -> &AuthData {
        match &self.extra {
            Some(extra) => &extra.auth_provider,
            None => &AUTH_DATA_NONE,
        }
    }

    pub fn get_libraries_with_overrides(&self) -> Vec<Library> {
        self.base
            .iter()
            .flat_map(|metadata| with_overrides(&metadata.libraries, &metadata.id))
            .collect()
    }

    pub fn get_id(&self) -> &str {
        &self.base.last().unwrap().id
    }

    pub fn get_parent_id(&self) -> &str {
        &self.base[0].id
    }

    pub fn get_asset_index(&self) -> BoxResult<&AssetIndex> {
        Ok(self.base[0]
            .asset_index
            .as_ref()
            .ok_or(VersionMetadataError::MissingAssetIndex)?)
    }

    pub fn get_arguments(&self) -> BoxResult<Arguments> {
        let mut merged_arguments = self.base[0].get_arguments()?;

        for metadata in &self.base[1..] {
            if let Some(arguments) = metadata.arguments.clone() {
                merged_arguments.game.extend(arguments.game);
                merged_arguments.jvm.extend(arguments.jvm);
            } else if metadata.minecraft_arguments.is_some() {
                merged_arguments = metadata.get_arguments()?;
            }
        }

        Ok(merged_arguments)
    }

    pub fn get_main_class(&self) -> &str {
        self.base.last().unwrap().main_class.as_str()
    }

    pub fn get_extra(&self) -> Option<&ExtraVersionMetadata> {
        self.extra.as_ref()
    }

    pub fn get_extra_forge_libs(&self) -> Vec<Library> {
        self.extra
            .as_ref()
            .map(|extra| extra.extra_forge_libs.clone())
            .unwrap_or_default()
    }
}
