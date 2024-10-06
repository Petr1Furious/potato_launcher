use egui::Widget as _;
use shared::paths::get_java_dir;
use std::path::Path;
use std::sync::{mpsc, Arc};
use tokio::runtime::Runtime;
use tokio_util::sync::CancellationToken;

use crate::config::runtime_config;
use crate::lang::{Lang, LangMessage};
use crate::utils;
use crate::version::complete_version_metadata::CompleteVersionMetadata;

use shared::java;
use shared::progress::{ProgressBar, Unit};

use super::progress_bar::GuiProgressBar;
use super::task::Task;

#[derive(Clone, PartialEq)]
pub enum JavaDownloadStatus {
    NotDownloaded,
    Downloading,
    Downloaded,
    DownloadError(String),
    DownloadErrorOffline,
}

pub struct JavaDownloadResult {
    pub status: JavaDownloadStatus,
    pub java_installation: Option<java::JavaInstallation>,
}

pub fn download_java(
    runtime: &Runtime,
    required_version: &str,
    java_dir: &Path,
    progress_bar: Arc<dyn ProgressBar<LangMessage>>,
    cancellation_token: CancellationToken,
) -> Task<JavaDownloadResult> {
    progress_bar.set_message(LangMessage::DownloadingJava);

    let (tx, rx) = mpsc::channel();

    let required_version = required_version.to_string();
    let java_dir = java_dir.to_path_buf();

    runtime.spawn(async move {
        let fut = java::download_java(&required_version, &java_dir, progress_bar.clone());

        let result = tokio::select! {
            _ = cancellation_token.cancelled() => JavaDownloadResult {
                status: JavaDownloadStatus::NotDownloaded,
                java_installation: None,
            },
            res = fut => match res {
                Ok(java_installation) => JavaDownloadResult {
                    status: JavaDownloadStatus::Downloaded,
                    java_installation: Some(java_installation),
                },
                Err(e) => JavaDownloadResult {
                    status: if utils::is_connect_error(&e) {
                        JavaDownloadStatus::DownloadErrorOffline
                    } else {
                        JavaDownloadStatus::DownloadError(e.to_string())
                    },
                    java_installation: None,
                },
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
    cancellation_token: CancellationToken,
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
            cancellation_token: CancellationToken::new(),
        }
    }

    fn check_java(
        &mut self,
        metadata: &CompleteVersionMetadata,
        config: &mut runtime_config::Config,
    ) {
        if let Some(java_path) = config.java_paths.get(metadata.get_name()) {
            if !java::check_java(&metadata.get_java_version(), java_path.as_ref()) {
                config.java_paths.remove(metadata.get_name());
                runtime_config::save_config(config);
            }
        }

        if config.java_paths.get(metadata.get_name()).is_none() {
            let launcher_dir = runtime_config::get_launcher_dir(config);

            if let Some(java_installation) =
                java::get_java(&metadata.get_java_version(), &get_java_dir(&launcher_dir))
            {
                config.java_paths.insert(
                    metadata.get_name().to_string(),
                    java_installation.path.to_str().unwrap().to_string(),
                );
                runtime_config::save_config(config);
            }
        }
    }

    pub fn update(
        &mut self,
        runtime: &Runtime,
        metadata: &CompleteVersionMetadata,
        config: &mut runtime_config::Config,
        need_java_check: bool,
    ) {
        if need_java_check {
            self.check_java(metadata, config);
            if config.java_paths.get(metadata.get_name()).is_some() {
                self.status = JavaDownloadStatus::Downloaded;
            }
            if config.java_paths.get(metadata.get_name()).is_none()
                && self.status == JavaDownloadStatus::Downloaded
            {
                self.status = JavaDownloadStatus::NotDownloaded;
            }

            self.settings_opened = false;
        }

        if self.status == JavaDownloadStatus::Downloading && self.java_download_task.is_none() {
            let launcher_dir = runtime_config::get_launcher_dir(config);
            let java_dir = get_java_dir(&launcher_dir);

            self.java_download_progress_bar.reset();
            self.cancellation_token = CancellationToken::new();

            let java_download_task = download_java(
                runtime,
                &metadata.get_java_version(),
                &java_dir,
                self.java_download_progress_bar.clone(),
                self.cancellation_token.clone(),
            );
            self.java_download_task = Some(java_download_task);
        }

        if let Some(task) = self.java_download_task.as_ref() {
            if let Some(result) = task.take_result() {
                self.status = result.status;
                self.java_download_task = None;
                if self.status == JavaDownloadStatus::Downloaded {
                    let path = result.java_installation.as_ref().unwrap().path.clone();
                    if java::check_java(&metadata.get_java_version(), &path) {
                        config.java_paths.insert(
                            metadata.get_name().to_string(),
                            path.to_str().unwrap().to_string(),
                        );
                        runtime_config::save_config(config);
                    } else {
                        self.status = JavaDownloadStatus::DownloadError(
                            "Downloaded Java is not valid".to_string(),
                        );
                    }
                }
            }
        }
    }

    fn get_download_button_text(
        &self,
        selected_metadata: &CompleteVersionMetadata,
        config: &runtime_config::Config,
    ) -> egui::Button {
        egui::Button::new(
            LangMessage::DownloadJava {
                version: selected_metadata.get_java_version().clone(),
            }
            .to_string(&config.lang),
        )
    }

    fn is_download_needed(&self) -> bool {
        match self.status {
            JavaDownloadStatus::NotDownloaded => true,
            JavaDownloadStatus::DownloadError(_) => true,
            JavaDownloadStatus::DownloadErrorOffline => true,
            JavaDownloadStatus::Downloading => false,
            JavaDownloadStatus::Downloaded => false,
        }
    }

    pub fn schedule_download_if_needed(&mut self) {
        if self.is_download_needed() {
            self.status = JavaDownloadStatus::Downloading;
        }
    }

    pub fn render_ui(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut runtime_config::Config,
        selected_metadata: &CompleteVersionMetadata,
    ) {
        match self.status {
            JavaDownloadStatus::NotDownloaded => {
                ui.label(
                    LangMessage::NeedJava {
                        version: selected_metadata.get_java_version().clone(),
                    }
                    .to_string(&config.lang),
                );
            }
            JavaDownloadStatus::Downloading => {} // message is shown in progress bar
            JavaDownloadStatus::DownloadError(ref e) => {
                ui.label(LangMessage::ErrorDownloadingJava(e.clone()).to_string(&config.lang));
            }
            JavaDownloadStatus::DownloadErrorOffline => {
                ui.label(LangMessage::NoConnectionToJavaServer.to_string(&config.lang));
            }
            JavaDownloadStatus::Downloaded => {
                ui.label(
                    LangMessage::JavaInstalled {
                        version: selected_metadata.get_java_version().clone(),
                    }
                    .to_string(&config.lang),
                );
            }
        }

        if self.status == JavaDownloadStatus::Downloading {
            self.java_download_progress_bar.render(ui, &config.lang);
            self.render_cancel_button(ui, &config.lang);
        }

        if self.is_download_needed() {
            if self
                .get_download_button_text(selected_metadata, config)
                .ui(ui)
                .clicked()
            {
                self.status = JavaDownloadStatus::Downloading;
            }
        }
        if ui
            .button(LangMessage::JavaSettings.to_string(&config.lang))
            .clicked()
        {
            self.settings_opened = true;

            self.picked_java_path = config
                .java_paths
                .get(&selected_metadata.get_name().to_string())
                .cloned();
            self.selected_xmx = Some(config.xmx.clone());
        }

        self.render_settings_window(ui, config, selected_metadata);
    }

    fn render_settings_window(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut runtime_config::Config,
        selected_metadata: &CompleteVersionMetadata,
    ) {
        let mut update_status = false;
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
                        if java::check_java(&selected_metadata.get_java_version(), &path) {
                            self.picked_java_path = Some(path.display().to_string());
                            config.java_paths.insert(
                                selected_metadata.get_name().to_string(),
                                path.display().to_string(),
                            );
                            runtime_config::save_config(config);
                            update_status = true;
                        } else {
                            self.picked_java_path = None;
                        }
                    }
                }

                ui.label(LangMessage::JavaXMX.to_string(&config.lang));
                ui.text_edit_singleline(self.selected_xmx.as_mut().unwrap());

                if ui
                    .button(LangMessage::OpenLauncherDirectory.to_string(&config.lang))
                    .clicked()
                {
                    open::that(runtime_config::get_launcher_dir(config)).unwrap();
                }

                if utils::validate_xmx(self.selected_xmx.as_ref().unwrap())
                    && config.xmx != self.selected_xmx.as_ref().unwrap().as_str()
                {
                    config.xmx = self.selected_xmx.as_ref().unwrap().clone();
                    runtime_config::save_config(config);
                }
            });
        if update_status {
            self.status = JavaDownloadStatus::Downloaded;
        }
    }

    pub fn ready_for_launch(&self) -> bool {
        self.status == JavaDownloadStatus::Downloaded
    }

    fn render_cancel_button(&mut self, ui: &mut egui::Ui, lang: &Lang) {
        if ui
            .button(LangMessage::CancelDownload.to_string(lang))
            .clicked()
        {
            self.cancel_download();
        }
    }

    pub fn cancel_download(&mut self) {
        self.cancellation_token.cancel();
    }
}
