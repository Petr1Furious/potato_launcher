use std::path::Path;
use std::sync::{mpsc, Arc};
use egui::Widget;
use tokio::runtime::Runtime;

use crate::config::runtime_config;
use crate::lang::LangMessage;
use crate::launcher::java;
use crate::modpack::index::ModpackIndex;
use crate::progress::ProgressBar;

use super::progress_bar::GuiProgressBar;
use super::task::Task;

#[derive(Clone, PartialEq)]
pub enum JavaDownloadStatus {
    NotDownloaded,
    Downloaded,
    DownloadError(String),
}

pub struct JavaDownloadResult {
    pub status: JavaDownloadStatus,
    pub java_installation: Option<java::JavaInstallation>,
}

pub fn download_java(
    runtime: &Runtime,
    required_version: &str,
    java_dir: &Path,
    progress_bar: Arc<dyn ProgressBar>,
) -> Task<JavaDownloadResult> {
    let (tx, rx) = mpsc::channel();

    let required_version = required_version.to_string();
    let java_dir = java_dir.to_path_buf();

    runtime.spawn(async move {
        let result =
            match java::download_java(&required_version, &java_dir, progress_bar.clone())
                .await
            {
                Ok(java_installation) => JavaDownloadResult {
                    status: JavaDownloadStatus::Downloaded,
                    java_installation: Some(java_installation),
                },
                Err(e) => JavaDownloadResult {
                    status: JavaDownloadStatus::DownloadError(e.to_string()),
                    java_installation: None,
                },
            };

        let _ = tx.send(result);
        progress_bar.finish();
    });

    return Task::new(rx);
}

pub struct JavaState {
    status: JavaDownloadStatus,
    java_download_task: Option<Task<JavaDownloadResult>>,
    java_download_progress_bar: Arc<GuiProgressBar>,
}

impl JavaState {
    pub fn new(ctx: &egui::Context) -> Self {
        Self {
            status: JavaDownloadStatus::NotDownloaded,
            java_download_task: None,
            java_download_progress_bar: Arc::new(GuiProgressBar::new(ctx)),
        }
    }

    fn check_java(&mut self, index: &ModpackIndex, config: &mut runtime_config::Config) {
        if let Some(java_path) = config.java_paths.get(&index.modpack_name) {
            if !java::check_java(&index.java_version, java_path.as_ref()) {
                config.java_paths.remove(&index.modpack_name);
                runtime_config::save_config(config);
            }
        }

        if config.java_paths.get(&index.modpack_name).is_none() {
            if let Some(java_installation) = java::get_java(&index.java_version, &runtime_config::get_java_dir(config)) {
                config.java_paths.insert(index.modpack_name.clone(), java_installation.path.to_str().unwrap().to_string());
            }
        }
    }

    pub fn update(&mut self, runtime: &Runtime, index: &ModpackIndex, config: &mut runtime_config::Config, need_java_check: bool) {
        if need_java_check {
            self.status = JavaDownloadStatus::NotDownloaded;
        }

        if self.status == JavaDownloadStatus::NotDownloaded && self.java_download_task.is_none() {
            if need_java_check || config.java_paths.get(&index.modpack_name).is_none() {
                self.check_java(index, config);
            }

            if config.java_paths.get(&index.modpack_name).is_some() {
                self.status = JavaDownloadStatus::Downloaded;
            }

            if config.java_paths.get(&index.modpack_name).is_none() {
                let java_dir = runtime_config::get_java_dir(config);
                let java_download_progress_bar = self.java_download_progress_bar.clone();
                let java_download_task = download_java(runtime, &index.java_version, &java_dir, java_download_progress_bar);
                self.java_download_progress_bar.reset();
                self.java_download_task = Some(java_download_task);
            }
        }

        if let Some(task) = self.java_download_task.as_ref() {
            if let Some(result) = task.take_result() {
                if result.status == JavaDownloadStatus::Downloaded {
                    config.java_paths.insert(index.modpack_name.clone(), result.java_installation.as_ref().unwrap().path.to_str().unwrap().to_string());
                    runtime_config::save_config(config);
                }
                self.status = result.status;
                self.java_download_task = None;
            }
        }
    }

    pub fn render_ui(&mut self, ui: &mut egui::Ui, config: &runtime_config::Config, selected_index: &ModpackIndex) {
        if self.java_download_task.is_some() {
            let progress_bar_state = self.java_download_progress_bar.get_state();
            if let Some(message) = &progress_bar_state.message {
                ui.label(message.to_string(&config.lang));
            }
            egui::ProgressBar::new(progress_bar_state.progress as f32 / progress_bar_state.total as f32)
                .text(format!("{} / {}", &progress_bar_state.progress, &progress_bar_state.total))
                .ui(ui);
        } else if self.status != JavaDownloadStatus::Downloaded {
            if ui.button(LangMessage::DownloadJava{ version: selected_index.java_version.clone() }.to_string(&config.lang)).clicked() {
                self.status = JavaDownloadStatus::NotDownloaded;
            }
        }

        if config.java_paths.get(&selected_index.modpack_name).is_some() {
            ui.label(LangMessage::JavaInstalled{ version: selected_index.java_version.clone() }.to_string(&config.lang));
        }
    }

    pub fn ready_for_launch(&self) -> bool {
        self.status == JavaDownloadStatus::Downloaded
    }
}
