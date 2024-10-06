use shared::paths::get_manifest_path;
use std::sync::{mpsc, Arc};
use tokio::runtime::Runtime;
use tokio_util::sync::CancellationToken;

use crate::config::runtime_config;
use crate::lang::{Lang, LangMessage};
use crate::utils;
use crate::version::complete_version_metadata::CompleteVersionMetadata;
use crate::version::sync;

use shared::progress::ProgressBar;
use shared::version::version_manifest::{self, VersionInfo, VersionManifest};

use super::progress_bar::GuiProgressBar;
use super::task::Task;

#[derive(Clone, PartialEq)]
enum ModpackSyncStatus {
    NotSynced,
    Syncing {
        ignore_version: bool,
        force_overwrite: bool,
    },
    Synced,
    SyncError(String),
    SyncErrorOffline,
}

struct ModpackSyncResult {
    status: ModpackSyncStatus,
}

fn sync_modpack(
    runtime: &Runtime,
    modpack_metadata: Arc<CompleteVersionMetadata>,
    force_overwrite: bool,
    sync_data: sync::SyncData,
    progress_bar: Arc<dyn ProgressBar<LangMessage>>,
    cancellation_token: CancellationToken,
) -> Task<ModpackSyncResult> {
    progress_bar.set_message(LangMessage::CheckingFiles);

    let (tx, rx) = mpsc::channel();

    let modpack_metadata = modpack_metadata.clone();

    runtime.spawn(async move {
        let fut = sync::sync_modpack(
            &modpack_metadata,
            force_overwrite,
            sync_data,
            progress_bar.clone(),
        );

        let result = tokio::select! {
            _ = cancellation_token.cancelled() => ModpackSyncResult {
                status: ModpackSyncStatus::NotSynced,
            },
            res = fut => match res {
                Ok(()) => ModpackSyncResult {
                    status: ModpackSyncStatus::Synced,
                },
                Err(e) => ModpackSyncResult {
                    status: if utils::is_connect_error(&e) {
                        ModpackSyncStatus::SyncErrorOffline
                    } else {
                        ModpackSyncStatus::SyncError(e.to_string())
                    },
                },
            }
        };

        let _ = tx.send(result);
        progress_bar.finish();
    });

    return Task::new(rx);
}

pub struct ModpackSyncState {
    status: ModpackSyncStatus,
    modpack_sync_task: Option<Task<ModpackSyncResult>>,
    modpack_sync_progress_bar: Arc<GuiProgressBar>,
    local_version_manifest: VersionManifest,
    modpack_sync_window_open: bool,
    force_overwrite_checked: bool,
    cancellation_token: CancellationToken,
}

pub enum UpdateResult {
    ModpackSyncComplete,
    ModpackNotSynced,
}

impl ModpackSyncState {
    pub async fn new(ctx: &egui::Context, config: &runtime_config::Config) -> Self {
        let modpack_sync_progress_bar = Arc::new(GuiProgressBar::new(ctx));

        let launcher_dir = runtime_config::get_launcher_dir(config);

        return ModpackSyncState {
            status: ModpackSyncStatus::NotSynced,
            modpack_sync_task: None,
            modpack_sync_progress_bar,
            local_version_manifest: version_manifest::load_local_version_manifest_safe(
                &get_manifest_path(&launcher_dir),
            )
            .await,
            modpack_sync_window_open: false,
            force_overwrite_checked: false,
            cancellation_token: CancellationToken::new(),
        };
    }

    fn is_up_to_date(&self, selected_version: &VersionInfo) -> bool {
        self.local_version_manifest
            .versions
            .iter()
            .find(|i| i.get_name() == selected_version.get_name())
            .is_some()
    }

    pub fn update(
        &mut self,
        runtime: &Runtime,
        selected_version_info: &VersionInfo,
        selected_version_metadata: Arc<CompleteVersionMetadata>,
        config: &runtime_config::Config,
        need_modpack_check: bool,
        online_manifest: bool,
    ) -> UpdateResult {
        if need_modpack_check {
            self.status = ModpackSyncStatus::NotSynced;
        }

        if self.status == ModpackSyncStatus::NotSynced {
            if self.is_up_to_date(selected_version_info) && online_manifest {
                self.status = ModpackSyncStatus::Synced;
            }
        }

        if let ModpackSyncStatus::Syncing {
            ignore_version,
            force_overwrite,
        } = self.status.clone()
        {
            if self.modpack_sync_task.is_none() {
                if !ignore_version {
                    if self.is_up_to_date(selected_version_info) {
                        self.status = ModpackSyncStatus::Synced;
                    }
                }

                if self.status != ModpackSyncStatus::Synced {
                    let launcher_dir = runtime_config::get_launcher_dir(config);
                    let assets_dir = runtime_config::get_assets_dir(config);

                    self.cancellation_token = CancellationToken::new();
                    self.modpack_sync_progress_bar.reset();

                    let path_data = sync::SyncData {
                        launcher_dir,
                        assets_dir,
                        version_name: selected_version_info.get_name(),
                    };
                    self.modpack_sync_task = Some(sync_modpack(
                        runtime,
                        selected_version_metadata,
                        force_overwrite,
                        path_data,
                        self.modpack_sync_progress_bar.clone(),
                        self.cancellation_token.clone(),
                    ));
                }
            }
        }

        if let Some(task) = self.modpack_sync_task.as_ref() {
            if let Some(result) = task.take_result() {
                self.status = result.status;
                self.modpack_sync_task = None;
                self.modpack_sync_window_open = false;

                if self.status == ModpackSyncStatus::Synced {
                    self.local_version_manifest
                        .versions
                        .retain(|i| i.get_name() != selected_version_info.get_name());
                    self.local_version_manifest
                        .versions
                        .push(selected_version_info.clone());

                    let launcher_dir = runtime_config::get_launcher_dir(config);

                    let _ = runtime.block_on(version_manifest::save_local_version_manifest(
                        &self.local_version_manifest,
                        &get_manifest_path(&launcher_dir),
                    ));
                }

                if self.status != ModpackSyncStatus::NotSynced {
                    return UpdateResult::ModpackSyncComplete;
                }
            }
        }
        UpdateResult::ModpackNotSynced
    }

    pub fn schedule_sync_if_needed(&mut self) {
        let need_sync = match &self.status {
            ModpackSyncStatus::NotSynced => true,
            ModpackSyncStatus::SyncError(_) => true,
            ModpackSyncStatus::SyncErrorOffline => true,
            ModpackSyncStatus::Syncing {
                ignore_version: _,
                force_overwrite: _,
            } => false,
            ModpackSyncStatus::Synced => false,
        };
        if need_sync {
            self.status = ModpackSyncStatus::Syncing {
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
        ui.label(match &self.status {
            ModpackSyncStatus::NotSynced => LangMessage::ModpackNotSynced.to_string(&config.lang),
            ModpackSyncStatus::Syncing {
                ignore_version: _,
                force_overwrite: _,
            } => LangMessage::SyncingModpack.to_string(&config.lang),
            ModpackSyncStatus::Synced => LangMessage::ModpackSynced.to_string(&config.lang),
            ModpackSyncStatus::SyncError(e) => {
                LangMessage::ModpackSyncError(e.clone()).to_string(&config.lang)
            }
            ModpackSyncStatus::SyncErrorOffline => {
                LangMessage::NoConnectionToSyncServer.to_string(&config.lang)
            }
        });

        if !manifest_online {
            return;
        }
        if ui
            .button(LangMessage::SyncModpack.to_string(&config.lang))
            .clicked()
        {
            if self.status == ModpackSyncStatus::NotSynced {
                self.status = ModpackSyncStatus::Syncing {
                    ignore_version: false,
                    force_overwrite: false,
                };
            } else {
                self.modpack_sync_window_open = true;
            }
        }

        if self.modpack_sync_window_open {
            let mut modpack_sync_window_open = self.modpack_sync_window_open.clone();
            egui::Window::new(LangMessage::SyncModpack.to_string(&config.lang))
                .open(&mut modpack_sync_window_open)
                .show(ui.ctx(), |ui| {
                    ui.checkbox(
                        &mut self.force_overwrite_checked,
                        LangMessage::ForceOverwrite.to_string(&config.lang),
                    );
                    ui.label(LangMessage::ForceOverwriteWarning.to_string(&config.lang));

                    if ui
                        .button(LangMessage::SyncModpack.to_string(&config.lang))
                        .clicked()
                    {
                        self.status = ModpackSyncStatus::Syncing {
                            ignore_version: true,
                            force_overwrite: self.force_overwrite_checked,
                        };
                    }

                    if self.modpack_sync_task.is_some() {
                        self.modpack_sync_progress_bar.render(ui, &config.lang);
                        self.render_cancel_button(ui, &config.lang);
                    }
                });
            self.modpack_sync_window_open = modpack_sync_window_open;
        } else {
            if self.modpack_sync_task.is_some() {
                self.modpack_sync_progress_bar.render(ui, &config.lang);
                self.render_cancel_button(ui, &config.lang);
            }
        }
    }

    pub fn ready_for_launch(&self) -> bool {
        self.status == ModpackSyncStatus::Synced
    }

    fn render_cancel_button(&mut self, ui: &mut egui::Ui, lang: &Lang) {
        if ui
            .button(LangMessage::CancelDownload.to_string(lang))
            .clicked()
        {
            self.cancel_sync();
        }
    }

    pub fn cancel_sync(&mut self) {
        self.cancellation_token.cancel();
    }
}
