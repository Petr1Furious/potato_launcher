use tokio::{process::Child, runtime::Runtime};

use crate::{
    config::runtime_config, lang::LangMessage, launcher::launch, modpack::index::ModpackIndex,
};

enum LauncherStatus {
    NotLaunched,
    Running { child: Child },
    Error(String),
}

pub struct Launcher {
    status: LauncherStatus,
}

impl Launcher {
    pub fn new() -> Self {
        Launcher {
            status: LauncherStatus::NotLaunched,
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
                if ui
                    .button(LangMessage::Launch.to_string(&config.lang))
                    .clicked()
                {
                    self.launch(runtime, config, selected_modpack.clone(), online);
                }

                ui.checkbox(
                    &mut config.close_launcher_after_launch,
                    LangMessage::CloseLauncherAfterLaunch.to_string(&config.lang),
                );
            }
        }

        match &self.status {
            LauncherStatus::Error(e) => {
                ui.label(LangMessage::LaunchError(e.clone()).to_string(&config.lang));
            }
            _ => {}
        }
    }
}

impl Drop for Launcher {
    fn drop(&mut self) {}
}
