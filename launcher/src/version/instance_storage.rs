use std::{collections::HashSet, path::Path};

use log::{error, warn};
use serde::{Deserialize, Serialize};
use shared::{
    paths::{
        get_instance_dir, get_instance_meta_path, get_instances_dir, get_local_instances_path,
        get_minecraft_dir,
    },
    version::version_manifest::{VersionInfo, VersionManifest},
};
use tokio::task;

use crate::{
    config::{build_config, runtime_config::Config},
    utils::get_temp_dir,
};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum InstanceStatus {
    Missing,
    Outdated,
    UpToDate,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LocalInstance {
    pub version_info: VersionInfo,
    pub status: InstanceStatus,
    pub manifest_url: Option<String>,
}

pub struct InstanceStorage {
    instances: Vec<LocalInstance>,
    remote_manifest: Option<VersionManifest>,
    remote_manifest_url: Option<String>,
}

impl InstanceStorage {
    async fn migrate_from_local_instances(launcher_dir: &std::path::Path) -> Vec<LocalInstance> {
        let local_instances_path = get_local_instances_path(launcher_dir);
        if !local_instances_path.exists() {
            return Vec::new();
        }
        match tokio::fs::read(&local_instances_path).await {
            Ok(data) => match serde_json::from_slice::<Vec<LocalInstance>>(&data) {
                Ok(mut from_legacy) => {
                    for instance in &mut from_legacy {
                        instance.manifest_url =
                            Some(build_config::get_default_version_manifest_url());
                        let meta_path =
                            get_instance_meta_path(launcher_dir, &instance.version_info.get_name());
                        if let Ok(serialized) = serde_json::to_vec_pretty(instance)
                            && let Err(e) = tokio::fs::write(&meta_path, serialized).await {
                                warn!(
                                    "Failed to write per-instance meta for {}: {e:?}",
                                    instance.version_info.get_name()
                                );
                            }
                    }
                    if let Err(e) = tokio::fs::remove_file(&local_instances_path).await {
                        warn!("Failed to remove legacy local_instances.json: {e:?}");
                    }
                    from_legacy
                }
                Err(e) => {
                    warn!("Failed to parse legacy local_instances.json, skipping migration: {e:?}");
                    Vec::new()
                }
            },
            Err(e) => {
                warn!("Failed to read legacy local_instances.json, skipping migration: {e:?}");
                Vec::new()
            }
        }
    }

    pub async fn load(config: &Config) -> InstanceStorage {
        let launcher_dir = config.get_launcher_dir();
        let mut instances: Vec<LocalInstance> = Vec::new();
        let instances_dir = get_instances_dir(&launcher_dir);
        let mut read_dir = match tokio::fs::read_dir(&instances_dir).await {
            Ok(rd) => rd,
            Err(_) => {
                if let Err(e) = tokio::fs::create_dir_all(&instances_dir).await {
                    error!("Failed to create instances directory: {e:?}");
                }
                match tokio::fs::read_dir(&instances_dir).await {
                    Ok(rd) => rd,
                    Err(e) => {
                        error!("Failed to read instances directory: {e:?}");
                        return InstanceStorage {
                            instances: Vec::new(),
                            remote_manifest: None,
                            remote_manifest_url: None,
                        };
                    }
                }
            }
        };
        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let path = entry.path();
            if path.is_dir()
                && let Some(name_os) = path.file_name() {
                    let dir_name = name_os.to_string_lossy().to_string();
                    let meta_path = get_instance_meta_path(&launcher_dir, &dir_name);
                    if meta_path.exists() {
                        if let Ok(data) = tokio::fs::read(&meta_path).await
                            && let Ok(instance) = serde_json::from_slice::<LocalInstance>(&data) {
                                instances.push(instance);
                            }
                        continue;
                    }

                    let mut dir_read = match tokio::fs::read_dir(&path).await {
                        Ok(rd) => rd,
                        Err(e) => {
                            warn!("Failed to read instance dir {:?}: {e:?}", &path);
                            continue;
                        }
                    };
                    let mut entries_to_move = Vec::new();
                    while let Ok(Some(child)) = dir_read.next_entry().await {
                        let child_path = child.path();
                        if let Some(child_name) = child_path.file_name()
                            && child_name != "minecraft" {
                                entries_to_move.push((child_name.to_os_string(), child_path));
                            }
                    }
                    let mc_dir = get_minecraft_dir(&launcher_dir, &dir_name);
                    for (name, from_path) in entries_to_move {
                        let to_path = mc_dir.join(name);
                        if let Err(e) = tokio::fs::rename(&from_path, &to_path).await {
                            warn!("Failed to move {:?} -> {:?}: {e:?}", from_path, to_path);
                        }
                    }
                }
        }

        if instances.is_empty() {
            let migrated = Self::migrate_from_local_instances(&launcher_dir).await;
            if !migrated.is_empty() {
                instances = migrated;
            }
        }

        InstanceStorage {
            instances,
            remote_manifest: None,
            remote_manifest_url: None,
        }
    }

    pub async fn safe_save(&self, config: &Config) {
        let launcher_dir = config.get_launcher_dir();
        let mut join_set = task::JoinSet::new();
        for instance in self.instances.clone() {
            let launcher_dir = launcher_dir.clone();
            let name = instance.version_info.get_name().to_string();
            let data = match serde_json::to_vec_pretty(&instance) {
                Ok(d) => d,
                Err(e) => {
                    error!("Failed to serialize instance {}: {e:?}", name);
                    continue;
                }
            };
            join_set.spawn(async move {
                let meta_path = get_instance_meta_path(&launcher_dir, &name);
                if let Err(e) = tokio::fs::write(&meta_path, data).await {
                    error!("Failed to save instance meta for {}: {e:?}", name);
                }
            });
        }
        while let Some(res) = join_set.join_next().await {
            let _ = res;
        }
    }

    pub fn set_remote_manifest(&mut self, manifest: VersionManifest, manifest_url: &str) {
        self.remote_manifest = Some(manifest);
        self.remote_manifest_url = Some(manifest_url.to_string());
    }

    fn get_remote_versions(&self) -> Vec<&VersionInfo> {
        if let Some(remote_manifest) = &self.remote_manifest {
            remote_manifest.versions.iter().collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_all_names_for_manifest_url(&self, url: &str) -> (Vec<String>, Vec<String>) {
        let local_names: HashSet<String> = self
            .instances
            .iter()
            .filter(|instance| instance.manifest_url.as_deref().unwrap_or(url) == url)
            .map(|x| x.version_info.get_name())
            .collect();
        let remote_names: Vec<String> = self
            .get_remote_versions()
            .iter()
            .map(|x| x.get_name())
            .filter(|x| !local_names.contains(x))
            .collect();

        let mut local_names: Vec<String> = local_names.into_iter().collect();
        local_names.sort();

        (local_names, remote_names)
    }

    pub fn count_instances_with_manifest_url(&self, url: &str) -> usize {
        self.instances
            .iter()
            .filter(|i| i.manifest_url.as_deref() == Some(url))
            .count()
    }

    pub async fn add_local_instance(&mut self, config: &Config, version_info: VersionInfo) {
        self.instances.push(LocalInstance {
            version_info,
            status: InstanceStatus::Outdated,
            manifest_url: None,
        });
        self.safe_save(config).await;
    }

    pub fn get_instance(&self, version_name: &str) -> Option<LocalInstance> {
        let local_instance = self
            .instances
            .iter()
            .find(|instance| instance.version_info.get_name() == version_name)
            .cloned();
        if let Some(local_instance) = &local_instance
            && let Some(manifest_url) = self.remote_manifest_url.clone()
                && let Some(instance_manifest_url) = local_instance.manifest_url.clone()
                && manifest_url != instance_manifest_url {
                    return Some(local_instance.clone()); // TODO: allow different manifest urls for the same instance name
                }
        let remote_version_info = self
            .get_remote_versions()
            .into_iter()
            .find(|x| x.get_name() == version_name);

        if let Some(remote_version_info) = remote_version_info {
            let remote_instance = LocalInstance {
                version_info: remote_version_info.clone(),
                status: if let Some(instance) = local_instance {
                    if remote_version_info != &instance.version_info {
                        InstanceStatus::Outdated
                    } else {
                        InstanceStatus::UpToDate
                    }
                } else {
                    InstanceStatus::Missing
                },
                manifest_url: self.remote_manifest_url.clone(),
            };
            Some(remote_instance)
        } else {
            local_instance
        }
    }

    pub async fn mark_downloaded(&mut self, config: &Config, version_name: &str) {
        let remote_versions = self.get_remote_versions();
        let remote_version = remote_versions
            .into_iter()
            .find(|v| v.get_name() == version_name)
            .cloned();

        if let Some(remote_version) = remote_version {
            self.instances
                .retain(|instance| instance.version_info.get_name() != version_name);
            self.instances.push(LocalInstance {
                version_info: remote_version,
                status: InstanceStatus::UpToDate,
                manifest_url: self.remote_manifest_url.clone(),
            });
            self.safe_save(config).await;
        } else if let Some(instance) = self
            .instances
            .iter_mut()
            .find(|instance| instance.version_info.get_name() == version_name)
        {
            instance.status = InstanceStatus::UpToDate;
            self.safe_save(config).await;
        } else {
            warn!("Tried to mark non-existent version as downloaded: {version_name}");
        }
    }

    async fn remove_instance_files(&self, launcher_dir: &Path, version_name: &str) {
        let instance_dir = get_instance_dir(launcher_dir, version_name);
        if instance_dir.exists() {
            let unique_temp_dir;
            let mut i = 0;
            loop {
                let temp_dir = get_temp_dir().join(format!("{version_name}_{i}"));
                if !temp_dir.exists() {
                    unique_temp_dir = temp_dir;
                    break;
                }
                i += 1;
            }

            if let Err(e) = tokio::fs::rename(&instance_dir, &unique_temp_dir).await {
                error!("Error moving instance directory:\n{e:?}");
            } else {
                task::spawn(async move {
                    if let Err(e) = tokio::fs::remove_dir_all(&unique_temp_dir).await {
                        error!("Error deleting temporary directory:\n{e:?}");
                    }
                });
            }
        }
    }

    pub async fn delete_instance(&mut self, config: &Config, version_name: &str) {
        let launcher_dir = config.get_launcher_dir();

        let instance = self
            .instances
            .iter()
            .find(|instance| instance.version_info.get_name() == version_name);
        if let Some(instance) = instance {
            if instance.status != InstanceStatus::Outdated {
                self.remove_instance_files(&launcher_dir, version_name)
                    .await;
            }
            self.instances
                .retain(|instance| instance.version_info.get_name() != version_name);
            self.safe_save(config).await;
        }
    }
}
