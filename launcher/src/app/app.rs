use eframe::egui;
use eframe::run_native;
use tokio::runtime::Runtime;

use super::auth_state::AuthState;
use super::instance_sync_state::InstanceSyncState;
use super::java_state::JavaState;
use super::launch_state::ForceLaunchResult;
use super::launch_state::LaunchState;
use super::manifest_state::ManifestState;
use super::metadata_state::MetadataState;
use super::new_instance_state::NewInstanceState;
use super::settings::SettingsState;
use crate::config::build_config;
use crate::config::runtime_config;
use crate::utils;

pub struct LauncherApp {
    runtime: Runtime,
    config: runtime_config::Config,
    settings_state: SettingsState,
    auth_state: AuthState,
    manifest_state: ManifestState,
    metadata_state: MetadataState,
    java_state: JavaState,
    instance_sync_state: InstanceSyncState,
    launch_state: LaunchState,
    new_instance_state: NewInstanceState,
}

pub fn run_gui(config: runtime_config::Config) {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size((650.0, 500.0))
            .with_icon(utils::get_icon_data())
            .with_resizable(false),
        ..Default::default()
    };

    run_native(
        &build_config::get_launcher_name(),
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

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.window_fill.to_normalized_gamma_f32()
    }
}

impl LauncherApp {
    fn new(config: runtime_config::Config, ctx: &egui::Context) -> Self {
        let runtime = Runtime::new().unwrap();

        LauncherApp {
            settings_state: SettingsState::new(),
            auth_state: AuthState::new(ctx),
            manifest_state: ManifestState::new(&runtime, &config, ctx),
            metadata_state: MetadataState::new(),
            java_state: JavaState::new(ctx),
            instance_sync_state: InstanceSyncState::new(ctx),
            launch_state: LaunchState::new(),
            new_instance_state: NewInstanceState::new(&runtime, ctx),
            runtime,
            config,
        }
    }

    fn ui(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    let selected_metadata = self.metadata_state.get_version_metadata();
                    let selected_metadata_ref = selected_metadata.as_deref();
                    self.settings_state.render_ui(
                        ui,
                        &self.runtime,
                        &mut self.config,
                        selected_metadata_ref,
                    );
                });
                ui.add_space(5.0);
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::central_panel(&ctx.style())
                    .outer_margin(egui::Margin::symmetric(150.0, 100.0))
                    .stroke(ctx.style().visuals.widgets.noninteractive.bg_stroke)
                    .rounding(egui::Rounding::same(10.0)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    let mut need_check = false;

                    need_check |= self.manifest_state.update(&mut self.config);

                    if let Some(new_instance) = self.new_instance_state.take_new_instance() {
                        self.manifest_state.add_new_version(
                            &self.runtime,
                            &mut self.config,
                            new_instance,
                        );
                    }

                    ui.horizontal(|ui| {
                        need_check |= self.manifest_state.render_combo_box(ui, &mut self.config);
                        self.new_instance_state.render_ui(
                            &self.runtime,
                            ui,
                            &mut self.config,
                            &self.manifest_state.get_all_names(),
                        );
                    });
                    self.manifest_state.render_status(ui, &self.config);
                    ui.separator();

                    if let Some(selected_instance) =
                        self.manifest_state.get_selected_instance(&self.config)
                    {
                        if need_check {
                            self.metadata_state.reset();
                        }

                        need_check |= self.metadata_state.update(
                            &self.runtime,
                            &mut self.config,
                            selected_instance,
                            ctx,
                        );

                        if self.metadata_state.render_ui(ui, &self.config) {
                            ui.separator();
                        }

                        if let Some(version_metadata) = self.metadata_state.get_version_metadata() {
                            if need_check {
                                self.auth_state
                                    .reset_auth_if_needed(version_metadata.get_auth_data());
                            }
                            need_check |= self
                                .auth_state
                                .update(&mut self.config, version_metadata.get_auth_data());

                            if need_check {
                                self.instance_sync_state.reset_status();
                            }

                            self.auth_state.render_ui(
                                ui,
                                &self.config,
                                &self.runtime,
                                ctx,
                                version_metadata.get_auth_data(),
                            );

                            if self
                                .auth_state
                                .ready_for_launch(version_metadata.get_auth_data(), &self.config)
                            {
                                let manifest_online =
                                    self.manifest_state.online() && self.metadata_state.online();
                                let instance_synced = self.instance_sync_state.update(
                                    &self.runtime,
                                    self.manifest_state.get_local_manifest(),
                                    selected_instance,
                                    version_metadata.clone(),
                                    &self.config,
                                    manifest_online,
                                );
                                if instance_synced {
                                    self.manifest_state.add_new_version(
                                        &self.runtime,
                                        &mut self.config,
                                        selected_instance.clone(),
                                    );
                                }

                                need_check |= instance_synced;

                                if need_check {
                                    self.java_state.set_check_java_task(
                                        &self.runtime,
                                        &version_metadata,
                                        &mut self.config,
                                        ctx,
                                    );
                                }
                                self.java_state.update(&version_metadata, &mut self.config);

                                ui.separator();
                                self.instance_sync_state.render_ui(
                                    ui,
                                    &mut self.config,
                                    manifest_online,
                                );

                                ui.separator();
                                self.java_state
                                    .render_ui(ui, &mut self.config, &version_metadata);

                                if self.java_state.ready_for_launch()
                                    && (self.instance_sync_state.ready_for_launch()
                                        || !manifest_online)
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
                                            self.instance_sync_state.schedule_sync_if_needed();
                                            self.java_state.schedule_download_if_needed(
                                                &self.runtime,
                                                &version_metadata,
                                                &mut self.config,
                                            );
                                        }
                                        ForceLaunchResult::CancelSelected => {
                                            self.java_state.cancel_download();
                                            self.instance_sync_state.cancel_sync();
                                        }
                                        ForceLaunchResult::NotSelected => {}
                                    }
                                }
                            }
                        }
                    }
                });
            });
    }
}
