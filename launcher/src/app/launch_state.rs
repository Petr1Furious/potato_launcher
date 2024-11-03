use egui::WidgetText;
use shared::paths::get_logs_dir;
use tokio::{process::Child, runtime::Runtime};

use crate::{
    config::runtime_config, lang::LangMessage, launcher::launch,
    version::complete_version_metadata::CompleteVersionMetadata,
};

enum LauncherStatus {
    NotLaunched,
    Running { child: Child },
    Error(String),
    ProcessErrorCode(String),
}

pub struct LaunchState {
    status: LauncherStatus,
    force_launch: bool,
}

pub enum ForceLaunchResult {
    NotSelected,
    ForceLaunchSelected,
    CancelSelected,
}

impl LaunchState {
    pub fn new() -> Self {
        LaunchState {
            status: LauncherStatus::NotLaunched,
            force_launch: false,
        }
    }

    fn launch(
        &mut self,
        runtime: &Runtime,
        config: &runtime_config::Config,
        selected_instance: &CompleteVersionMetadata,
        online: bool,
    ) {
        match runtime.block_on(launch::launch(selected_instance, config, online)) {
            Ok(child) => {
                if config.close_launcher_after_launch {
                    std::process::exit(0);
                }
                self.status = LauncherStatus::Running { child };
            }
            Err(e) => {
                self.status = LauncherStatus::Error(e.to_string());
            }
        }
    }

    pub fn update(&mut self) {
        match self.status {
            LauncherStatus::Running { ref mut child } => {
                match child.try_wait() {
                    Ok(Some(exit_status)) => {
                        self.status = if exit_status.success() {
                            LauncherStatus::NotLaunched
                        } else {
                            LauncherStatus::ProcessErrorCode(
                                exit_status.code().unwrap_or(-1).to_string(),
                            )
                        };
                    }
                    Ok(None) => {}
                    Err(e) => {
                        self.status = LauncherStatus::Error(e.to_string());
                    }
                };
            }
            _ => {}
        }
    }

    fn big_button_clicked(ui: &mut egui::Ui, text: &str) -> bool {
        let widget_text = WidgetText::from(text).text_style(egui::TextStyle::Button);
        let button = egui::Button::new(widget_text);
        ui.add_sized([150.0, 35.0], button).clicked()
    }

    pub fn render_ui(
        &mut self,
        runtime: &Runtime,
        ui: &mut egui::Ui,
        config: &mut runtime_config::Config,
        selected_instance: &CompleteVersionMetadata,
        online: bool,
    ) {
        match &mut self.status {
            LauncherStatus::Running { child } => {
                ui.label(LangMessage::Running.to_string(config.lang));
                if ui
                    .button(LangMessage::KillMinecraft.to_string(config.lang))
                    .clicked()
                {
                    let _ = runtime.block_on(child.kill());
                }
            }
            _ => {
                if self.force_launch
                    || LaunchState::big_button_clicked(
                        ui,
                        &LangMessage::Launch.to_string(config.lang),
                    )
                {
                    self.force_launch = false;
                    self.launch(runtime, config, selected_instance, online);
                }
            }
        }

        match &self.status {
            LauncherStatus::Error(e) => {
                ui.label(LangMessage::LaunchError(e.clone()).to_string(config.lang));
            }
            LauncherStatus::ProcessErrorCode(e) => {
                ui.label(LangMessage::ProcessErrorCode(e.clone()).to_string(config.lang));
                if ui
                    .button(LangMessage::OpenLogs.to_string(config.lang))
                    .clicked()
                {
                    open::that(get_logs_dir(&config.get_launcher_dir())).unwrap();
                }
            }
            _ => {}
        }
    }

    pub fn render_download_ui(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut runtime_config::Config,
    ) -> ForceLaunchResult {
        if !self.force_launch {
            if LaunchState::big_button_clicked(
                ui,
                &LangMessage::DownloadAndLaunch.to_string(config.lang),
            ) {
                self.force_launch = true;
                return ForceLaunchResult::ForceLaunchSelected;
            }
        } else {
            let mut cancel_clicked = false;
            if LaunchState::big_button_clicked(
                ui,
                &LangMessage::CancelLaunch.to_string(config.lang),
            ) {
                self.force_launch = false;
                cancel_clicked = true;
            }
            if cancel_clicked {
                return ForceLaunchResult::CancelSelected;
            }
        }
        ForceLaunchResult::NotSelected
    }
}

impl Drop for LaunchState {
    fn drop(&mut self) {}
}
