use egui::Widget;
use std::path::Path;
use std::sync::{mpsc, Arc};
use tokio::runtime::Runtime;

use crate::config::runtime_config;
use crate::lang::LangMessage;
use crate::launcher::java;
use crate::modpack::index::ModpackIndex;
use crate::progress::{ProgressBar, Unit};

use super::progress_bar::GuiProgressBar;
use super::task::Task;

#[derive(Clone, PartialEq)]
pub enum JavaDownloadStatus {
    NotDownloaded,
    NeedDownload,
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
            match java::download_java(&required_version, &java_dir, progress_bar.clone()).await {
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
    settings_opened: bool,
    picked_java_path: Option<String>,
    selected_xmx: Option<String>,
}

impl JavaState {
    pub fn new(ctx: &egui::Context) -> Self {
        let java_download_progress_bar = Arc::new(GuiProgressBar::new(ctx));
        java_download_progress_bar.set_unit(Unit {
            name: "MB".to_string(),
            size: 1024 * 1024,
        });
        Self {
            status: JavaDownloadStatus::NotDownloaded,
            java_download_task: None,
            java_download_progress_bar,
            settings_opened: false,
            picked_java_path: None,
            selected_xmx: None,
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
            if let Some(java_installation) =
                java::get_java(&index.java_version, &runtime_config::get_java_dir(config))
            {
                config.java_paths.insert(
                    index.modpack_name.clone(),
                    java_installation.path.to_str().unwrap().to_string(),
                );
            }
        }
    }

    pub fn update(
        &mut self,
        runtime: &Runtime,
        index: &ModpackIndex,
        config: &mut runtime_config::Config,
        need_java_check: bool,
    ) {
        if need_java_check {
            self.java_download_task = None;
            self.check_java(index, config);
            if config.java_paths.get(&index.modpack_name).is_some() {
                self.status = JavaDownloadStatus::Downloaded;
            } else {
                self.status = JavaDownloadStatus::NotDownloaded;
            }

            self.settings_opened = false;
        }

        if self.status == JavaDownloadStatus::NeedDownload && self.java_download_task.is_none() {
            if config.java_paths.get(&index.modpack_name).is_none() {
                let java_dir = runtime_config::get_java_dir(config);
                let java_download_task = download_java(
                    runtime,
                    &index.java_version,
                    &java_dir,
                    self.java_download_progress_bar.clone(),
                );
                self.java_download_progress_bar.reset();
                self.java_download_task = Some(java_download_task);
            }
        }

        if let Some(task) = self.java_download_task.as_ref() {
            if let Some(result) = task.take_result() {
                if result.status == JavaDownloadStatus::Downloaded {
                    config.java_paths.insert(
                        index.modpack_name.clone(),
                        result
                            .java_installation
                            .as_ref()
                            .unwrap()
                            .path
                            .to_str()
                            .unwrap()
                            .to_string(),
                    );
                    runtime_config::save_config(config);
                }
                self.status = result.status;
                self.java_download_task = None;
            }
        }
    }

    fn get_download_button_text(
        &self,
        selected_index: &ModpackIndex,
        config: &runtime_config::Config,
    ) -> egui::Button {
        egui::Button::new(
            LangMessage::DownloadJava {
                version: selected_index.java_version.clone(),
            }
            .to_string(&config.lang),
        )
    }

    pub fn render_ui(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut runtime_config::Config,
        selected_index: &ModpackIndex,
    ) {
        match self.status {
            JavaDownloadStatus::NotDownloaded => {
                ui.label(
                    LangMessage::NeedJava {
                        version: selected_index.java_version.clone(),
                    }
                    .to_string(&config.lang),
                );
            }
            JavaDownloadStatus::NeedDownload => {} // message is shown in progress bar
            JavaDownloadStatus::DownloadError(ref e) => {
                ui.label(LangMessage::ErrorDownloadingJava(e.clone()).to_string(&config.lang));
            }
            JavaDownloadStatus::Downloaded => {
                ui.label(
                    LangMessage::JavaInstalled {
                        version: selected_index.java_version.clone(),
                    }
                    .to_string(&config.lang),
                );
            }
        }

        let mut show_download_button = false;
        if self.java_download_task.is_some() {
            self.java_download_progress_bar.render(ui, &config.lang);
        } else if self.status != JavaDownloadStatus::Downloaded {
            show_download_button = true;
        }

        if show_download_button {
            if self
                .get_download_button_text(selected_index, config)
                .ui(ui)
                .clicked()
            {
                self.status = JavaDownloadStatus::NeedDownload;
            }
        }
        if ui
            .button(LangMessage::JavaSettings.to_string(&config.lang))
            .clicked()
        {
            self.settings_opened = true;

            self.picked_java_path = Some(
                config
                    .java_paths
                    .get(&selected_index.modpack_name)
                    .unwrap()
                    .clone(),
            );
            self.selected_xmx = Some(config.xmx.clone());
        }

        self.render_settings_window(ui, config, selected_index);
    }

    fn render_settings_window(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut runtime_config::Config,
        selected_index: &ModpackIndex,
    ) {
        egui::Window::new(LangMessage::JavaSettings.to_string(&config.lang))
            .open(&mut self.settings_opened)
            .show(ui.ctx(), |ui| {
                ui.label(
                    LangMessage::SelectedJavaPath {
                        path: self.picked_java_path.clone(),
                    }
                    .to_string(&config.lang),
                );

                if ui
                    .button(LangMessage::SelectJavaPath.to_string(&config.lang))
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        if java::check_java(&selected_index.java_version, &path) {
                            self.picked_java_path = Some(path.display().to_string());
                            config.java_paths.insert(
                                selected_index.modpack_name.clone(),
                                path.display().to_string(),
                            );
                            runtime_config::save_config(config);
                        } else {
                            self.picked_java_path = None;
                        }
                    }
                }

                ui.label(LangMessage::JavaXMX.to_string(&config.lang));
                ui.text_edit_singleline(self.selected_xmx.as_mut().unwrap());

                if runtime_config::validate_xmx(self.selected_xmx.as_ref().unwrap())
                    && config.xmx != self.selected_xmx.as_ref().unwrap().as_str()
                {
                    config.xmx = self.selected_xmx.as_ref().unwrap().clone();
                    runtime_config::save_config(config);
                }
            });
    }

    pub fn ready_for_launch(&self) -> bool {
        self.status == JavaDownloadStatus::Downloaded
    }
}
