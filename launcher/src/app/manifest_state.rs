use crate::{config::runtime_config::Config, lang::LangMessage};

use egui::RichText;
use log::error;
use shared::utils::is_connect_error;
use shared::version::version_manifest::VersionManifest;
use tokio::runtime::Runtime;

use super::{
    background_task::{BackgroundTask, BackgroundTaskResult},
    colors,
};

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
    url: String,
    callback: Callback,
) -> BackgroundTask<ManifestFetchResult>
where
    Callback: FnOnce() + Send + 'static,
{
    let fut = async move {
        let result = VersionManifest::fetch(&url).await;
        match result {
            Ok(manifest) => ManifestFetchResult {
                status: FetchStatus::Fetched,
                manifest: Some(manifest),
            },
            Err(e) => ManifestFetchResult {
                status: if is_connect_error(&e) {
                    FetchStatus::FetchErrorOffline
                } else {
                    error!("Error fetching version manifest:\n{e:?}");
                    FetchStatus::FetchError(e.to_string())
                },
                manifest: None,
            },
        }
    };

    BackgroundTask::with_callback(fut, runtime, Box::new(callback))
}

pub struct ManifestState {
    status: FetchStatus,
    fetch_task: Option<BackgroundTask<ManifestFetchResult>>,
}

impl ManifestState {
    fn set_fetch_task(&mut self, runtime: &Runtime, config: &Config, ctx: &egui::Context) {
        let ctx = ctx.clone();
        let url = config.get_effective_version_manifest_url();
        self.fetch_task = Some(fetch_manifest(runtime, url.to_string(), move || {
            ctx.request_repaint();
        }));
    }

    pub fn new(runtime: &Runtime, ctx: &egui::Context, config: &Config) -> ManifestState {
        let mut result = ManifestState {
            status: FetchStatus::NotFetched,
            fetch_task: None,
        };
        result.set_fetch_task(runtime, config, ctx);

        result
    }

    pub fn take_manifest(&mut self, config: &mut Config) -> (Option<VersionManifest>, bool) {
        if let Some(task) = self.fetch_task.as_ref()
            && task.has_result()
        {
            let task = self.fetch_task.take().unwrap();
            let result = task.take_result();
            match result {
                BackgroundTaskResult::Finished(result) => {
                    if let Some(manifest) = &result.manifest
                        && config.selected_instance_name.is_none()
                        && manifest.versions.len() == 1
                    {
                        config.selected_instance_name =
                            manifest.versions.first().map(|x| x.get_name());
                        config.save();
                    }
                    self.status = result.status;

                    return (result.manifest, true);
                }
                BackgroundTaskResult::Cancelled => {
                    self.status = FetchStatus::NotFetched;
                }
            }
        }

        (None, false)
    }

    pub fn render_combo_box(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut Config,
        local_instance_names: &Vec<String>,
        remote_instance_names: &Vec<String>,
    ) -> bool {
        let mut selected_instance_name = config.selected_instance_name.clone();
        let dark_mode = ui.style().visuals.dark_mode;

        ui.horizontal(|ui| {
            let selected_text = if let Some(instance_text) = config.selected_instance_name.clone() {
                match self.status {
                    FetchStatus::NotFetched => RichText::new(format!(
                        "{} ({})",
                        instance_text,
                        LangMessage::FetchingRemote.to_string(config.lang)
                    ))
                    .color(colors::in_progress(dark_mode)),
                    FetchStatus::Fetched => {
                        RichText::new(instance_text).color(colors::ok(dark_mode))
                    }
                    FetchStatus::FetchErrorOffline => RichText::new(format!(
                        "{} ({})",
                        instance_text,
                        LangMessage::Offline.to_string(config.lang)
                    ))
                    .color(colors::offline(dark_mode)),
                    FetchStatus::FetchError(_) => RichText::new(format!(
                        "{} ({})",
                        instance_text,
                        LangMessage::ErrorFetchingRemote.to_string(config.lang)
                    ))
                    .color(colors::error(dark_mode)),
                }
            } else {
                RichText::new(LangMessage::SelectInstance.to_string(config.lang))
                    .color(colors::action(dark_mode))
            };

            egui::ComboBox::from_id_salt("instances")
                .width(ui.available_width())
                .selected_text(selected_text)
                .show_ui(ui, |ui| {
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

    pub fn retry_fetch(&mut self, runtime: &Runtime, config: &Config, ctx: &egui::Context) {
        self.status = FetchStatus::NotFetched;
        self.set_fetch_task(runtime, config, ctx);
    }

    pub fn online(&self) -> bool {
        self.status == FetchStatus::Fetched
    }

    pub fn is_fetching(&self) -> bool {
        self.fetch_task.is_some()
    }
}
