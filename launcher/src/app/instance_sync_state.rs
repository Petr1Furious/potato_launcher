use egui::RichText;
use log::error;
use shared::progress::ProgressBar;
use shared::utils::is_connect_error;
use std::path::Path;
use std::sync::Arc;
use tokio::runtime::Runtime;

use crate::config::runtime_config::Config;
use crate::lang::{Lang, LangMessage};
use crate::version::complete_version_metadata::CompleteVersionMetadata;
use crate::version::sync;

use super::background_task::{BackgroundTask, BackgroundTaskResult};
use super::colors;
use super::progress_bar::GuiProgressBar;

#[derive(Clone, PartialEq)]
enum InstanceSyncStatus {
    NotSynced,
    Synced,
    SyncError,
    SyncErrorOffline,
}

fn sync_instance(
    runtime: &Runtime,
    instance_metadata: Arc<CompleteVersionMetadata>,
    force_overwrite: bool,
    launcher_dir: &Path,
    assets_dir: &Path,
    progress_bar: Arc<dyn ProgressBar<LangMessage>>,
) -> BackgroundTask<anyhow::Result<()>> {
    let launcher_dir = launcher_dir.to_path_buf();
    let assets_dir = assets_dir.to_path_buf();

    let instance_metadata = instance_metadata.clone();
    let progress_bar_clone = progress_bar.clone();
    let fut = async move {
        progress_bar_clone.set_message(LangMessage::CheckingFiles);
        sync::sync_instance(
            &instance_metadata,
            force_overwrite,
            &launcher_dir,
            &assets_dir,
            progress_bar_clone,
        )
        .await
    };

    BackgroundTask::with_callback(
        fut,
        runtime,
        Box::new(move || {
            progress_bar.finish();
        }),
    )
}

pub struct InstanceSyncState {
    status: InstanceSyncStatus,
    instance_sync_task: Option<BackgroundTask<anyhow::Result<()>>>,
    instance_sync_progress_bar: Arc<GuiProgressBar>,

    instance_sync_window_open: bool,
    force_overwrite_checked: bool,
}

impl InstanceSyncState {
    pub fn new(ctx: &egui::Context) -> Self {
        let instance_sync_progress_bar = Arc::new(GuiProgressBar::new(ctx));

        InstanceSyncState {
            status: InstanceSyncStatus::NotSynced,
            instance_sync_task: None,
            instance_sync_progress_bar,

            instance_sync_window_open: false,
            force_overwrite_checked: false,
        }
    }

    pub fn update(&mut self) -> bool {
        if let Some(task) = self.instance_sync_task.as_ref()
            && task.has_result()
        {
            self.instance_sync_window_open = false;
            let task = self.instance_sync_task.take();
            match task.unwrap().take_result() {
                BackgroundTaskResult::Finished(result) => {
                    self.status = match result {
                        Ok(()) => InstanceSyncStatus::Synced,
                        Err(e) => {
                            if is_connect_error(&e) {
                                InstanceSyncStatus::SyncErrorOffline
                            } else {
                                error!("Error syncing instance:\n{e:?}");
                                InstanceSyncStatus::SyncError
                            }
                        }
                    };
                }
                BackgroundTaskResult::Cancelled => {
                    self.status = InstanceSyncStatus::NotSynced;
                }
            }

            if self.status == InstanceSyncStatus::Synced {
                return true;
            }
        }

        false
    }

    pub fn reset_status(&mut self) {
        self.status = InstanceSyncStatus::NotSynced;
    }

    pub fn set_up_to_date(&mut self) {
        self.status = InstanceSyncStatus::Synced;
    }

    fn schedule_sync(
        &mut self,
        runtime: &Runtime,
        selected_version_metadata: Arc<CompleteVersionMetadata>,
        force_overwrite: bool,
        config: &Config,
        ctx: &egui::Context,
    ) {
        self.instance_sync_progress_bar = Arc::new(GuiProgressBar::new(ctx));
        if let Some(mut task) = self.instance_sync_task.take() {
            task.cancel();
        }
        self.instance_sync_task = Some(sync_instance(
            runtime,
            selected_version_metadata,
            force_overwrite,
            &config.get_launcher_dir(),
            &config.get_assets_dir(),
            self.instance_sync_progress_bar.clone(),
        ));
    }

    pub fn schedule_sync_if_needed(
        &mut self,
        runtime: &Runtime,
        selected_version_metadata: Arc<CompleteVersionMetadata>,
        force_overwrite: bool,
        config: &Config,
        ctx: &egui::Context,
    ) {
        match &self.status {
            InstanceSyncStatus::NotSynced
            | InstanceSyncStatus::SyncError
            | InstanceSyncStatus::SyncErrorOffline => {
                self.schedule_sync(
                    runtime,
                    selected_version_metadata,
                    force_overwrite,
                    config,
                    ctx,
                );
            }
            InstanceSyncStatus::Synced => {}
        };
    }

    pub fn render_status(&self, ui: &mut egui::Ui, config: &Config) {
        let lang = config.lang;
        let dark_mode = ui.style().visuals.dark_mode;

        ui.label(match &self.status {
            InstanceSyncStatus::NotSynced => {
                RichText::new(LangMessage::InstanceNotSynced.to_string(lang))
                    .color(colors::action(dark_mode))
            }
            InstanceSyncStatus::Synced => {
                RichText::new(LangMessage::InstanceSynced.to_string(lang))
                    .color(colors::ok(dark_mode))
            }
            InstanceSyncStatus::SyncError => {
                RichText::new(LangMessage::InstanceSyncError.to_string(lang))
                    .color(colors::error(dark_mode))
            }
            InstanceSyncStatus::SyncErrorOffline => {
                RichText::new(LangMessage::NoConnectionToSyncServer.to_string(lang))
                    .color(colors::offline(dark_mode))
            }
        });
    }

    pub fn render_windows(
        &mut self,
        ui: &mut egui::Ui,
        runtime: &Runtime,
        config: &Config,
        selected_version_metadata: Option<Arc<CompleteVersionMetadata>>,
    ) {
        self.render_sync_window(ui, runtime, config, selected_version_metadata);
        self.render_progress_bar_window(ui, config.lang);
    }

    pub fn render_sync_button(
        &mut self,
        ui: &mut egui::Ui,
        runtime: &Runtime,
        config: &Config,
        selected_version_metadata: Option<Arc<CompleteVersionMetadata>>,
    ) {
        let lang = config.lang;

        if ui
            .add_enabled(
                self.instance_sync_task.is_none()
                    && !self.instance_sync_window_open
                    && selected_version_metadata.is_some(),
                egui::Button::new(LangMessage::SyncInstance.to_string(lang)),
            )
            .clicked()
        {
            match &self.status {
                InstanceSyncStatus::NotSynced
                | InstanceSyncStatus::SyncError
                | InstanceSyncStatus::SyncErrorOffline => {
                    self.schedule_sync(
                        runtime,
                        selected_version_metadata.clone().unwrap(),
                        false,
                        config,
                        ui.ctx(),
                    );
                }
                _ => {
                    self.instance_sync_window_open = true;
                }
            }
        }
    }

    fn render_sync_window(
        &mut self,
        ui: &mut egui::Ui,
        runtime: &Runtime,
        config: &Config,
        selected_version_metadata: Option<Arc<CompleteVersionMetadata>>,
    ) {
        let lang = config.lang;
        let mut instance_sync_window_open = self.instance_sync_window_open;
        let mut close_sync_window = false;
        egui::Window::new(LangMessage::SyncInstance.to_string(lang))
            .open(&mut instance_sync_window_open)
            .show(ui.ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.checkbox(
                        &mut self.force_overwrite_checked,
                        LangMessage::ForceOverwrite.to_string(lang),
                    );
                    ui.label(LangMessage::ForceOverwriteWarning.to_string(lang));

                    if ui
                        .add_enabled(
                            selected_version_metadata.is_some(),
                            egui::Button::new(LangMessage::SyncInstance.to_string(lang)),
                        )
                        .clicked()
                    {
                        self.schedule_sync(
                            runtime,
                            selected_version_metadata.unwrap(),
                            self.force_overwrite_checked,
                            config,
                            ui.ctx(),
                        );
                        close_sync_window = true;
                    }
                });
            });
        self.instance_sync_window_open = instance_sync_window_open;
        if close_sync_window {
            self.instance_sync_window_open = false;
        }
    }

    fn render_progress_bar_window(&mut self, ui: &mut egui::Ui, lang: Lang) {
        if self.instance_sync_task.is_some() {
            egui::Window::new(LangMessage::InstanceSyncProgress.to_string(lang)).show(
                ui.ctx(),
                |ui| {
                    ui.vertical_centered(|ui| {
                        self.instance_sync_progress_bar.render(ui, lang);
                        self.render_cancel_button(ui, lang);
                    });
                },
            );
        }
    }

    fn render_cancel_button(&mut self, ui: &mut egui::Ui, lang: Lang) {
        if ui
            .button(LangMessage::CancelDownload.to_string(lang))
            .clicked()
        {
            self.cancel_sync();
        }
    }

    pub fn cancel_sync(&mut self) {
        if let Some(task) = self.instance_sync_task.as_mut() {
            task.cancel();
        }
    }

    pub fn is_syncing(&self) -> bool {
        self.instance_sync_task.is_some()
    }
}
