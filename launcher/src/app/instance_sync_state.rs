use shared::progress::ProgressBar;
use shared::utils::BoxResult;
use shared::version::version_manifest::{VersionInfo, VersionManifest};
use std::path::Path;
use std::sync::Arc;
use tokio::runtime::Runtime;

use crate::config::runtime_config;
use crate::lang::{Lang, LangMessage};
use crate::utils;
use crate::version::complete_version_metadata::CompleteVersionMetadata;
use crate::version::sync;

use super::background_task::{BackgroundTask, BackgroundTaskResult};
use super::progress_bar::GuiProgressBar;

#[derive(Clone, PartialEq)]
enum InstanceSyncStatus {
    NotSynced,
    Syncing {
        ignore_version: bool,
        force_overwrite: bool,
    },
    Synced,
    SyncError(String),
    SyncErrorOffline,
}

fn sync_instance(
    runtime: &Runtime,
    instance_metadata: Arc<CompleteVersionMetadata>,
    force_overwrite: bool,
    launcher_dir: &Path,
    assets_dir: &Path,
    progress_bar: Arc<dyn ProgressBar<LangMessage>>,
) -> BackgroundTask<BoxResult<()>> {
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
    instance_sync_task: Option<BackgroundTask<BoxResult<()>>>,
    instance_sync_progress_bar: Arc<GuiProgressBar>,

    instance_sync_window_open: bool,
    force_overwrite_checked: bool,
}

impl InstanceSyncState {
    pub fn new(ctx: &egui::Context) -> Self {
        let instance_sync_progress_bar = Arc::new(GuiProgressBar::new(ctx));

        return InstanceSyncState {
            status: InstanceSyncStatus::NotSynced,
            instance_sync_task: None,
            instance_sync_progress_bar,

            instance_sync_window_open: false,
            force_overwrite_checked: false,
        };
    }

    pub fn reset_status(&mut self) {
        self.status = InstanceSyncStatus::NotSynced;
    }

    pub fn update(
        &mut self,
        runtime: &Runtime,
        local_version_manifest: &VersionManifest,
        selected_version_info: &VersionInfo,
        selected_version_metadata: Arc<CompleteVersionMetadata>,
        config: &runtime_config::Config,
        online_manifest: bool,
    ) -> bool {
        if self.status == InstanceSyncStatus::NotSynced {
            if local_version_manifest.is_up_to_date(selected_version_info) && online_manifest {
                self.status = InstanceSyncStatus::Synced;
            }
        }

        if let InstanceSyncStatus::Syncing {
            ignore_version,
            force_overwrite,
        } = self.status.clone()
        {
            if self.instance_sync_task.is_none() {
                if !ignore_version {
                    if local_version_manifest.is_up_to_date(selected_version_info) {
                        self.status = InstanceSyncStatus::Synced;
                    }
                }

                if self.status != InstanceSyncStatus::Synced {
                    self.instance_sync_progress_bar.reset();
                    self.instance_sync_task = Some(sync_instance(
                        runtime,
                        selected_version_metadata,
                        force_overwrite,
                        &config.get_launcher_dir(),
                        &config.get_assets_dir(),
                        self.instance_sync_progress_bar.clone(),
                    ));
                }
            }
        }

        if let Some(task) = self.instance_sync_task.as_ref() {
            if task.has_result() {
                self.instance_sync_window_open = false;
                let task = self.instance_sync_task.take();
                match task.unwrap().take_result() {
                    BackgroundTaskResult::Finished(result) => {
                        self.status = match result {
                            Ok(()) => InstanceSyncStatus::Synced,
                            Err(e) => {
                                if utils::is_connect_error(&e) {
                                    InstanceSyncStatus::SyncErrorOffline
                                } else {
                                    InstanceSyncStatus::SyncError(e.to_string())
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
        }

        false
    }

    pub fn schedule_sync_if_needed(&mut self) {
        let need_sync = match &self.status {
            InstanceSyncStatus::NotSynced => true,
            InstanceSyncStatus::SyncError(_) => true,
            InstanceSyncStatus::SyncErrorOffline => true,
            InstanceSyncStatus::Syncing {
                ignore_version: _,
                force_overwrite: _,
            } => false,
            InstanceSyncStatus::Synced => false,
        };
        if need_sync {
            self.status = InstanceSyncStatus::Syncing {
                ignore_version: false,
                force_overwrite: false,
            };
        }
    }

    pub fn render_ui(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut runtime_config::Config,
        manifest_online: bool,
    ) {
        match &self.status {
            InstanceSyncStatus::NotSynced => {
                ui.label(LangMessage::InstanceNotSynced.to_string(config.lang));
            }
            InstanceSyncStatus::Syncing {
                ignore_version: _,
                force_overwrite: _,
            } => {
                // should be shown in the progress bar
            }
            InstanceSyncStatus::Synced => {
                ui.label(LangMessage::InstanceSynced.to_string(config.lang));
            }
            InstanceSyncStatus::SyncError(e) => {
                ui.label(LangMessage::InstanceSyncError(e.clone()).to_string(config.lang));
            }
            InstanceSyncStatus::SyncErrorOffline => {
                ui.label(LangMessage::NoConnectionToSyncServer.to_string(config.lang));
            }
        }

        if manifest_online {
            if self.instance_sync_task.is_some() && !self.instance_sync_window_open {
                self.instance_sync_progress_bar.render(ui, config.lang);
                self.render_cancel_button(ui, config.lang);
            } else {
                if ui
                    .button(LangMessage::SyncInstance.to_string(config.lang))
                    .clicked()
                {
                    match &self.status {
                        InstanceSyncStatus::NotSynced
                        | InstanceSyncStatus::SyncError(_)
                        | InstanceSyncStatus::SyncErrorOffline => {
                            self.status = InstanceSyncStatus::Syncing {
                                ignore_version: false,
                                force_overwrite: false,
                            };
                        }

                        _ => {
                            self.instance_sync_window_open = true;
                        }
                    }
                }

                self.render_sync_window(ui, config.lang);
            }
        }
    }

    pub fn render_sync_window(&mut self, ui: &mut egui::Ui, lang: Lang) {
        let mut instance_sync_window_open = self.instance_sync_window_open.clone();
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
                        .button(LangMessage::SyncInstance.to_string(lang))
                        .clicked()
                    {
                        self.status = InstanceSyncStatus::Syncing {
                            ignore_version: true,
                            force_overwrite: self.force_overwrite_checked,
                        };
                    }

                    if self.instance_sync_task.is_some() {
                        self.instance_sync_progress_bar.render(ui, lang);
                        self.render_cancel_button(ui, lang);
                    }
                });
            });
        self.instance_sync_window_open = instance_sync_window_open;
    }

    pub fn ready_for_launch(&self) -> bool {
        self.status == InstanceSyncStatus::Synced
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
        if let Some(task) = self.instance_sync_task.as_ref() {
            task.cancel();
        }
    }
}
