use std::path::Path;
use std::sync::{mpsc, Arc};
use tokio::runtime::Runtime;

use crate::config::runtime_config;
use crate::lang::LangMessage;
use crate::modpack::index::{self, ModpackIndex};
use crate::progress::ProgressBar;
use crate::utils;

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
    modpack_index: &ModpackIndex,
    force_overwrite: bool,
    modpack_dir: &Path,
    assets_dir: &Path,
    index_path: &Path,
    progress_bar: Arc<dyn ProgressBar>,
) -> Task<ModpackSyncResult> {
    progress_bar.set_message(LangMessage::CheckingFiles);

    let (tx, rx) = mpsc::channel();

    let modpack_index = modpack_index.clone();
    let modpack_dir = modpack_dir.to_path_buf();
    let assets_dir = assets_dir.to_path_buf();
    let index_path = index_path.to_path_buf();

    runtime.spawn(async move {
        let result = match index::sync_modpack(
            modpack_index,
            force_overwrite,
            &modpack_dir,
            &assets_dir,
            &index_path,
            progress_bar.clone(),
        )
        .await
        {
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
    local_indexes: Vec<ModpackIndex>,
    modpack_sync_window_open: bool,
    force_overwrite_checked: bool,
}

pub enum UpdateResult {
    ModpackSynced,
    ModpackNotSynced,
}

impl ModpackSyncState {
    pub fn new(ctx: &egui::Context, config: &runtime_config::Config) -> Self {
        let modpack_sync_progress_bar = Arc::new(GuiProgressBar::new(ctx));

        return ModpackSyncState {
            status: ModpackSyncStatus::NotSynced,
            modpack_sync_task: None,
            modpack_sync_progress_bar,
            local_indexes: index::load_local_indexes(&runtime_config::get_index_path(config)),
            modpack_sync_window_open: false,
            force_overwrite_checked: false,
        };
    }

    fn is_up_to_date(&self, selected_index: &ModpackIndex) -> bool {
        if let Some(local_index) = self
            .local_indexes
            .iter()
            .find(|i| i.modpack_name == selected_index.modpack_name)
        {
            return local_index.modpack_version == selected_index.modpack_version;
        }

        return false;
    }

    pub fn update(
        &mut self,
        runtime: &Runtime,
        selected_index: &ModpackIndex,
        config: &runtime_config::Config,
        need_modpack_check: bool,
        index_online: bool,
    ) -> UpdateResult {
        if need_modpack_check {
            self.status = ModpackSyncStatus::NotSynced;
        }

        if self.status == ModpackSyncStatus::NotSynced {
            if self.is_up_to_date(selected_index) && index_online {
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
                    if self.is_up_to_date(selected_index) {
                        self.status = ModpackSyncStatus::Synced;
                    }
                }

                if self.status != ModpackSyncStatus::Synced {
                    let modpack_dir =
                        runtime_config::get_minecraft_dir(config, &selected_index.modpack_name);
                    let assets_dir = runtime_config::get_assets_dir(config);
                    let index_path = runtime_config::get_index_path(config);

                    self.modpack_sync_progress_bar.reset();
                    self.modpack_sync_task = Some(sync_modpack(
                        runtime,
                        selected_index,
                        force_overwrite,
                        &modpack_dir,
                        &assets_dir,
                        &index_path,
                        self.modpack_sync_progress_bar.clone(),
                    ));
                }
            }
        }

        if let Some(task) = self.modpack_sync_task.as_ref() {
            if let Some(result) = task.take_result() {
                self.status = result.status;
                self.local_indexes =
                    index::load_local_indexes(&runtime_config::get_index_path(config));
                self.modpack_sync_task = None;
                self.modpack_sync_window_open = false;
                return UpdateResult::ModpackSynced;
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
        index_online: bool,
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

        if !index_online {
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
            egui::Window::new(LangMessage::SyncModpack.to_string(&config.lang))
                .open(&mut self.modpack_sync_window_open)
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
                    }
                });
        } else {
            if self.modpack_sync_task.is_some() {
                self.modpack_sync_progress_bar.render(ui, &config.lang);
            }
        }
    }

    pub fn ready_for_launch(&self) -> bool {
        self.status == ModpackSyncStatus::Synced
    }
}
