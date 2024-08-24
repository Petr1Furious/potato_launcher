use tokio::{process::Child, runtime::Runtime};

use crate::{config::runtime_config, lang::LangMessage, launcher::launch, modpack::index::ModpackIndex};

enum LauncherStatus {
    NotLaunched,
    Running{ child: Child },
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

    fn launch(&mut self, runtime: &Runtime, config: &runtime_config::Config, selected_modpack: ModpackIndex, online: bool) {
        match runtime.block_on(launch::launch(selected_modpack, config, online)) {
            Ok(child) => {
                self.status = LauncherStatus::Running{ child };
            },
            Err(e) => {
                self.status = LauncherStatus::Error(e.to_string());
            },
        }
    }

    pub fn update(&mut self) {
        match self.status {
            LauncherStatus::Running{ ref mut child } => {
                if child.try_wait().unwrap().is_some() {
                    self.status = LauncherStatus::NotLaunched;
                }
            },
            _ => {},
        }
    }

    pub fn render_ui(&mut self, runtime: &Runtime, ui: &mut egui::Ui, config: &runtime_config::Config, selected_modpack: &ModpackIndex, online: bool) {
        match &self.status {
            LauncherStatus::NotLaunched => {
                if ui.button(LangMessage::Launch.to_string(&config.lang)).clicked() {
                    self.launch(runtime, config, selected_modpack.clone(), online);
                }
            },
            LauncherStatus::Error(e) => {
                ui.label(LangMessage::LaunchError(e.clone()).to_string(&config.lang));
            },
            _ => {},
        }
    }
}
