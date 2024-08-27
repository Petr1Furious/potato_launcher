use eframe::egui;
use eframe::run_native;
use tokio::runtime::Runtime;

use super::auth_state::AuthState;
use super::index_state;
use super::index_state::IndexState;
use super::java_state::JavaState;
use super::language_selector::LanguageSelector;
use super::launcher::Launcher;
use super::modpack_sync_state::ModpackSyncState;
use crate::config::build_config;
use crate::config::runtime_config;

pub struct LauncherApp {
    runtime: Runtime,
    config: runtime_config::Config,
    language_selector: LanguageSelector,
    auth_state: AuthState,
    index_state: IndexState,
    java_state: JavaState,
    modpack_sync_state: ModpackSyncState,
    launcher: Launcher,
}

pub fn run_gui(config: runtime_config::Config) {
    let native_options = eframe::NativeOptions {
        ..Default::default()
    };

    run_native(
        &build_config::get_display_launcher_name(),
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(LauncherApp::new(config, &cc.egui_ctx)))
        }),
    )
    .unwrap();
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }
}

impl LauncherApp {
    fn new(config: runtime_config::Config, ctx: &egui::Context) -> Self {
        LauncherApp {
            runtime: Runtime::new().unwrap(),
            language_selector: LanguageSelector::new(),
            auth_state: AuthState::new(ctx),
            index_state: IndexState::new(),
            java_state: JavaState::new(ctx),
            modpack_sync_state: ModpackSyncState::new(ctx, &config),
            config,
            launcher: Launcher::new(),
        }
    }

    fn ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.language_selector.render_ui(ui, &mut self.config);

            self.auth_state.update(&self.runtime, &mut self.config);
            let update_result = self.index_state.update(&self.runtime, &self.config, ctx);

            let username = self.config.user_info.as_ref().map(|x| x.username.as_str());
            self.auth_state.render_ui(ui, &self.config.lang, username);

            let render_result = self.index_state.render_ui(ui, &mut self.config);
            let selected_modpack = self.index_state.get_selected_modpack(&self.config).cloned();
            if let Some(selected_modpack) = selected_modpack {
                let mut need_modpack_check = false;
                if let index_state::UpdateResult::IndexesUpdated = update_result {
                    need_modpack_check = true;
                } else if let index_state::UpdateResult::IndexesUpdated = render_result {
                    need_modpack_check = true;
                }

                let index_online = self.index_state.online();
                self.modpack_sync_state.update(
                    &self.runtime,
                    &selected_modpack,
                    &self.config,
                    need_modpack_check,
                    index_online,
                );
                self.java_state.update(
                    &self.runtime,
                    &selected_modpack,
                    &mut self.config,
                    need_modpack_check,
                );

                self.java_state
                    .render_ui(ui, &mut self.config, &selected_modpack);
                self.modpack_sync_state
                    .render_ui(ui, &mut self.config, index_online);

                if self.auth_state.ready_for_launch(&self.config)
                    && self.java_state.ready_for_launch()
                    && (self.modpack_sync_state.ready_for_launch() || !index_online)
                {
                    self.launcher.update();
                    self.launcher.render_ui(
                        &self.runtime,
                        ui,
                        &self.config,
                        &selected_modpack,
                        self.auth_state.online(),
                    );
                }
            }
        });
    }
}
