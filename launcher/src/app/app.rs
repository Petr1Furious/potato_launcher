use eframe::egui;
use eframe::run_native;
use tokio::runtime::Runtime;

use super::auth_state::AuthState;
use super::java_state::JavaState;
use super::language_selector::LanguageSelector;
use super::launch_state::ForceLaunchResult;
use super::launch_state::LaunchState;
use super::manifest_state;
use super::manifest_state::ManifestState;
use super::metadata_state;
use super::metadata_state::MetadataState;
use super::modpack_sync_state;
use super::modpack_sync_state::ModpackSyncState;
use crate::config::build_config;
use crate::config::runtime_config;
use crate::lang::LangMessage;
use crate::utils;

pub struct LauncherApp {
    runtime: Runtime,
    config: runtime_config::Config,
    language_selector: LanguageSelector,
    auth_state: AuthState,
    manifest_state: ManifestState,
    metadata_state: MetadataState,
    java_state: JavaState,
    modpack_sync_state: ModpackSyncState,
    launch_state: LaunchState,
}

pub fn run_gui(config: runtime_config::Config) {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size((400.0, 400.0))
            .with_icon(utils::get_icon_data()),
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
        let runtime = Runtime::new().unwrap();
        LauncherApp {
            language_selector: LanguageSelector::new(),
            auth_state: AuthState::new(ctx),
            manifest_state: ManifestState::new(),
            metadata_state: MetadataState::new(),
            java_state: JavaState::new(ctx),
            modpack_sync_state: runtime.block_on(ModpackSyncState::new(ctx, &config)),
            launch_state: LaunchState::new(),
            runtime,
            config,
        }
    }

    fn ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.language_selector.render_ui(ui, &mut self.config);

            self.auth_state.update(&self.runtime, &mut self.config);
            let manifest_fetch_result =
                self.manifest_state
                    .update(&self.runtime, &mut self.config, ctx);

            ui.heading(LangMessage::Authorization.to_string(&self.config.lang));

            let username = self.config.user_info.as_ref().map(|x| x.username.as_str());
            self.auth_state.render_ui(ui, &self.config.lang, username);

            ui.heading(LangMessage::Modpacks.to_string(&self.config.lang));

            let select_modpack_result = self.manifest_state.render_ui(ui, &mut self.config);
            let selected_modpack = self.manifest_state.get_selected_modpack(&self.config);
            if let Some(selected_modpack) = selected_modpack {
                let mut need_modpack_check = manifest_fetch_result
                    == manifest_state::UpdateResult::ManifestUpdated
                    || select_modpack_result == manifest_state::UpdateResult::ManifestUpdated;

                if need_modpack_check {
                    self.metadata_state.reset();
                }

                let metadata_get_result = self.metadata_state.update(
                    &self.runtime,
                    &mut self.config,
                    selected_modpack,
                    ctx,
                );
                self.metadata_state.render_ui(ui, &self.config);

                if let metadata_state::UpdateResult::MetadataUpdated = metadata_get_result {
                    need_modpack_check = true;
                }

                let version_metadata = self.metadata_state.get_version_metadata();
                if let Some(version_metadata) = version_metadata {
                    let manifest_online =
                        self.manifest_state.online() && self.metadata_state.online();
                    let update_result = self.modpack_sync_state.update(
                        &self.runtime,
                        selected_modpack,
                        version_metadata.clone(),
                        &self.config,
                        need_modpack_check,
                        manifest_online,
                    );
                    if let modpack_sync_state::UpdateResult::ModpackSyncComplete = update_result {
                        need_modpack_check = true;
                    }

                    self.java_state.update(
                        &self.runtime,
                        &version_metadata,
                        &mut self.config,
                        need_modpack_check,
                    );

                    self.modpack_sync_state
                        .render_ui(ui, &mut self.config, manifest_online);

                    self.java_state
                        .render_ui(ui, &mut self.config, &version_metadata);

                    if AuthState::ready_for_launch(&self.config) {
                        if self.java_state.ready_for_launch()
                            && (self.modpack_sync_state.ready_for_launch() || !manifest_online)
                        {
                            self.launch_state.update();
                            self.launch_state.render_ui(
                                &self.runtime,
                                ui,
                                &mut self.config,
                                &version_metadata,
                                self.auth_state.online(),
                            );
                        } else {
                            let force_launch_result =
                                self.launch_state.render_download_ui(ui, &mut self.config);
                            match force_launch_result {
                                ForceLaunchResult::ForceLaunchSelected => {
                                    self.modpack_sync_state.schedule_sync_if_needed();
                                    self.java_state.schedule_download_if_needed();
                                }
                                ForceLaunchResult::CancelSelected => {
                                    self.java_state.cancel_download();
                                    self.modpack_sync_state.cancel_sync();
                                }
                                ForceLaunchResult::NotSelected => {}
                            }
                        }
                    }
                }
            }

            ui.add_space(10.0);
        });
    }
}
