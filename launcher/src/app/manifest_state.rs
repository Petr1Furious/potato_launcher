use crate::{
    config::{build_config, runtime_config::Config},
    lang::LangMessage,
};

use shared::{
    paths::get_manifest_path,
    version::version_manifest::{VersionInfo, VersionManifest},
};
use tokio::runtime::Runtime;

use super::background_task::{BackgroundTask, BackgroundTaskResult};

#[derive(PartialEq)]
enum FetchStatus {
    NotFetched,
    Fetched,
    FetchErrorOffline,
    FetchError(String),
}

struct ManifestFetchResult {
    status: FetchStatus,
    manifest: Option<VersionManifest>,
}

fn fetch_manifest<Callback>(
    runtime: &tokio::runtime::Runtime,
    callback: Callback,
) -> BackgroundTask<ManifestFetchResult>
where
    Callback: FnOnce() + Send + 'static,
{
    let fut = async move {
        let result = VersionManifest::fetch(&build_config::get_version_manifest_url()).await;
        match result {
            Ok(manifest) => ManifestFetchResult {
                status: FetchStatus::Fetched,
                manifest: Some(manifest),
            },
            Err(e) => {
                let mut connect_error = false;
                if let Some(re) = e.downcast_ref::<reqwest::Error>() {
                    if re.is_connect() {
                        connect_error = true;
                    }
                }

                ManifestFetchResult {
                    status: if connect_error {
                        FetchStatus::FetchErrorOffline
                    } else {
                        FetchStatus::FetchError(e.to_string())
                    },
                    manifest: None,
                }
            }
        }
    };

    BackgroundTask::with_callback(fut, runtime, Box::new(callback))
}

pub struct ManifestState {
    status: FetchStatus,
    fetch_task: Option<BackgroundTask<ManifestFetchResult>>,
    remote_manifest: Option<VersionManifest>,
    local_manifest: VersionManifest,
}

impl ManifestState {
    fn set_fetch_task(&mut self, runtime: &Runtime, ctx: &egui::Context) {
        let ctx = ctx.clone();
        self.fetch_task = Some(fetch_manifest(runtime, move || {
            ctx.request_repaint();
        }));
    }

    pub fn new(runtime: &Runtime, config: &Config, ctx: &egui::Context) -> ManifestState {
        let launcher_dir = config.get_launcher_dir();
        let manifest_path = get_manifest_path(&launcher_dir);

        let mut result = ManifestState {
            status: FetchStatus::NotFetched,
            fetch_task: None,
            remote_manifest: None,
            local_manifest: runtime.block_on(VersionManifest::read_local_safe(&manifest_path)),
        };
        result.set_fetch_task(runtime, ctx);

        result
    }

    pub fn get_local_manifest(&self) -> &VersionManifest {
        &self.local_manifest
    }

    fn get_local_versions(&self) -> Vec<&VersionInfo> {
        self.local_manifest.versions.iter().collect()
    }

    fn get_remote_versions(&self) -> Vec<&VersionInfo> {
        if let Some(remote_manifest) = &self.remote_manifest {
            remote_manifest.versions.iter().collect()
        } else {
            Vec::new()
        }
    }

    fn get_names(versions: Vec<&VersionInfo>) -> Vec<String> {
        versions.iter().map(|x| x.get_name()).collect()
    }

    pub fn get_all_names(&self) -> Vec<String> {
        let mut names = Self::get_names(self.get_local_versions());
        let local_names = names.clone();
        names.extend(
            Self::get_names(self.get_remote_versions())
                .into_iter()
                .filter(|x| !local_names.contains(x)),
        );
        names
    }

    pub fn add_new_version(
        &mut self,
        runtime: &Runtime,
        config: &Config,
        version_info: VersionInfo,
    ) {
        let launcher_dir = config.get_launcher_dir();
        let manifest_path = get_manifest_path(&launcher_dir);
        let _ = runtime.block_on(
            self.local_manifest
                .add_version_and_save(version_info, &manifest_path),
        );
    }

    pub fn update(&mut self, config: &mut Config) -> bool {
        if let Some(task) = self.fetch_task.as_ref() {
            if task.has_result() {
                let task = self.fetch_task.take().unwrap();
                let result = task.take_result();
                match result {
                    BackgroundTaskResult::Finished(result) => {
                        if let Some(manifest) = &result.manifest {
                            if config.selected_instance_name.is_none()
                                && manifest.versions.len() == 1
                            {
                                config.selected_instance_name =
                                    manifest.versions.first().map(|x| x.get_name());
                                config.save();
                            }
                        }
                        self.status = result.status;
                        self.remote_manifest = result.manifest;
                    }
                    BackgroundTaskResult::Cancelled => {
                        self.status = FetchStatus::NotFetched;
                    }
                }

                return true;
            }
        }

        false
    }

    pub fn render_combo_box(&mut self, ui: &mut egui::Ui, config: &mut Config) -> bool {
        let mut selected_instance_name = config.selected_instance_name.clone();

        ui.horizontal(|ui| {
            ui.label(LangMessage::SelectInstance.to_string(config.lang));
            egui::ComboBox::from_id_source("instances")
                .selected_text(
                    selected_instance_name
                        .clone()
                        .unwrap_or_else(|| LangMessage::NotSelected.to_string(config.lang)),
                )
                .show_ui(ui, |ui| {
                    let local_instance_names: Vec<String> =
                        Self::get_names(self.get_local_versions());
                    let remote_instance_names: Vec<String> =
                        Self::get_names(self.get_remote_versions())
                            .into_iter()
                            .filter(|x| !local_instance_names.contains(x))
                            .collect();
                    if !local_instance_names.is_empty() || !remote_instance_names.is_empty() {
                        for instance_name in local_instance_names {
                            ui.selectable_value(
                                &mut selected_instance_name,
                                Some(instance_name.clone()),
                                instance_name,
                            );
                        }
                        for instance_name in remote_instance_names {
                            ui.selectable_value(
                                &mut selected_instance_name,
                                Some(instance_name.clone()),
                                egui::WidgetText::from(instance_name).italics(),
                            );
                        }
                    } else {
                        ui.label(LangMessage::NoInstances.to_string(config.lang));
                    }
                });
        });

        if config.selected_instance_name != selected_instance_name {
            config.selected_instance_name = selected_instance_name;
            config.save();
            true
        } else {
            false
        }
    }

    pub fn render_status(&mut self, ui: &mut egui::Ui, config: &Config) {
        match self.status {
            FetchStatus::NotFetched => {
                ui.label(LangMessage::FetchingVersionManifest.to_string(config.lang));
            }
            FetchStatus::Fetched => {}
            FetchStatus::FetchErrorOffline => {
                ui.label(LangMessage::NoConnectionToManifestServer.to_string(config.lang));
            }
            FetchStatus::FetchError(ref s) => {
                ui.label(
                    LangMessage::ErrorFetchingRemoteManifest(s.clone()).to_string(config.lang),
                );
            }
        }

        if self.status != FetchStatus::Fetched && self.status != FetchStatus::NotFetched {
            if ui
                .button(LangMessage::FetchManifest.to_string(config.lang))
                .clicked()
            {
                self.status = FetchStatus::NotFetched;
            }
        }
    }

    pub fn get_selected_instance(&self, config: &Config) -> Option<&VersionInfo> {
        if let Some(remote_manifest) = &self.remote_manifest {
            if let Some(version_info) = remote_manifest
                .versions
                .iter()
                .find(|x| Some(&x.get_name()) == config.selected_instance_name.as_ref())
            {
                return Some(version_info);
            }
        }

        if let Some(version_info) = self
            .local_manifest
            .versions
            .iter()
            .find(|x| Some(&x.get_name()) == config.selected_instance_name.as_ref())
        {
            return Some(version_info);
        }

        None
    }

    pub fn online(&self) -> bool {
        self.status == FetchStatus::Fetched
    }
}
