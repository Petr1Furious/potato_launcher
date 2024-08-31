use tokio::{process::Child, runtime::Runtime};

use crate::{
    config::runtime_config, lang::LangMessage, launcher::launch, modpack::index::ModpackIndex,
};

enum LauncherStatus {
    NotLaunched,
    Running { child: Child },
    Error(String),
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
        selected_modpack: ModpackIndex,
        online: bool,
    ) {
        match runtime.block_on(launch::launch(selected_modpack, config, online)) {
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
                if child.try_wait().unwrap().is_some() {
                    self.status = LauncherStatus::NotLaunched;
                }
            }
            _ => {}
        }
    }

    fn render_close_launcher_checkbox(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut runtime_config::Config,
    ) {
        let old_close_launcher_after_launch = config.close_launcher_after_launch;
        ui.checkbox(
            &mut config.close_launcher_after_launch,
            LangMessage::CloseLauncherAfterLaunch.to_string(&config.lang),
        );
        if old_close_launcher_after_launch != config.close_launcher_after_launch {
            runtime_config::save_config(config);
        }
    }

    pub fn render_ui(
        &mut self,
        runtime: &Runtime,
        ui: &mut egui::Ui,
        config: &mut runtime_config::Config,
        selected_modpack: &ModpackIndex,
        online: bool,
    ) {
        match &mut self.status {
            LauncherStatus::Running { child } => {
                ui.label(LangMessage::Running.to_string(&config.lang));
                if ui
                    .button(LangMessage::KillMinecraft.to_string(&config.lang))
                    .clicked()
                {
                    let _ = runtime.block_on(child.kill());
                }
            }
            _ => {
                if self.force_launch
                    || ui
                        .button(LangMessage::Launch.to_string(&config.lang))
                        .clicked()
                {
                    self.force_launch = false;
                    self.launch(runtime, config, selected_modpack.clone(), online);
                }

                self.render_close_launcher_checkbox(ui, config);
            }
        }

        match &self.status {
            LauncherStatus::Error(e) => {
                ui.label(LangMessage::LaunchError(e.clone()).to_string(&config.lang));
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
            if ui
                .button(LangMessage::DownloadAndLaunch.to_string(&config.lang))
                .clicked()
            {
                self.force_launch = true;
                return ForceLaunchResult::ForceLaunchSelected;
            }
        } else {
            let mut cancel_clicked = false;
            ui.horizontal(|ui| {
                if ui
                    .button(LangMessage::CancelLaunch.to_string(&config.lang))
                    .clicked()
                {
                    self.force_launch = false;
                    cancel_clicked = true;
                }
            });
            self.render_close_launcher_checkbox(ui, config);
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
