use super::language_selector::LanguageSelector;
use super::manifest_state::ManifestState;
use crate::config::build_config;
use crate::config::build_config::USE_NATIVE_GLFW_DEFAULT;
use crate::config::runtime_config::Config;
use crate::constants::{XMX_DEFAULT, XMX_MAX, XMX_MIN, XMX_STEP};
use crate::lang::LangMessage;
use crate::utils;
use crate::version::complete_version_metadata::CompleteVersionMetadata;
use crate::version::instance_storage::InstanceStorage;
use shared::java;
use tokio::runtime::Runtime;

fn get_xmx_max() -> f64 {
    utils::get_total_memory().map_or(XMX_MAX, |total| total / 1024) as f64
}

pub struct SettingsState {
    language_selector: LanguageSelector,
    settings_opened: bool,
    instance_settings_opened: bool,
    picked_java_path: Option<String>,
    xmx_slider_value: f64,
    use_native_glfw: bool,
    add_manifest_opened: bool,
    new_manifest_url: String,
}

fn map_xmx_slider_value(value: f64) -> String {
    let mb = utils::map_range(value, 0.0, 1.0, XMX_MIN as f64, get_xmx_max()) as u64;
    format!("{}M", ((mb + XMX_STEP / 2) / XMX_STEP) * XMX_STEP)
}

fn map_xmx_slider_value_reverse(value: &str) -> f64 {
    let xmx = value
        .trim_end_matches('M')
        .parse::<u64>()
        .unwrap_or(XMX_DEFAULT);
    utils::map_range(xmx as f64, XMX_MIN as f64, get_xmx_max(), 0.0, 1.0)
}

impl SettingsState {
    pub fn new() -> Self {
        Self {
            language_selector: LanguageSelector::new(),
            settings_opened: false,
            instance_settings_opened: false,
            picked_java_path: None,
            xmx_slider_value: 0.0,
            use_native_glfw: false,
            add_manifest_opened: false,
            new_manifest_url: String::new(),
        }
    }

    pub fn render_settings(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut Config,
        runtime: &Runtime,
        manifest_state: &mut ManifestState,
        ctx: &egui::Context,
        instance_storage: &InstanceStorage,
    ) {
        if ui.button("📂").clicked() {
            open::that(config.get_launcher_dir()).unwrap();
        }

        if ui
            .add_enabled(!self.settings_opened, egui::Button::new("⚙"))
            .clicked()
        {
            self.settings_opened = true;
        }

        self.language_selector.render_ui(ui, config);

        self.render_settings_window(ui, config, runtime, manifest_state, ctx, instance_storage);
    }

    pub fn render_settings_window(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut Config,
        runtime: &Runtime,
        manifest_state: &mut ManifestState,
        ctx: &egui::Context,
        instance_storage: &InstanceStorage,
    ) {
        let lang = config.lang;
        let mut settings_opened = self.settings_opened;

        egui::Window::new(LangMessage::Settings.to_string(lang))
            .open(&mut settings_opened)
            .show(ui.ctx(), |ui| {
                self.render_close_launcher_checkbox(ui, config);
                ui.separator();
                self.render_manifest_controls(
                    ui,
                    config,
                    runtime,
                    manifest_state,
                    ctx,
                    instance_storage,
                );
                self.render_add_manifest_window(ui, config);
            });

        self.settings_opened = settings_opened;
    }

    fn render_manifest_controls(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut Config,
        runtime: &Runtime,
        manifest_state: &mut ManifestState,
        ctx: &egui::Context,
        instance_storage: &InstanceStorage,
    ) {
        if ui
            .button(LangMessage::AddManifestUrl.to_string(config.lang))
            .clicked()
        {
            self.add_manifest_opened = true;
            self.new_manifest_url.clear();
        }

        if !config.extra_version_manifest_urls.is_empty() {
            ui.separator();
            let default_url = build_config::get_default_version_manifest_url();
            let current = config.get_effective_version_manifest_url();
            let before_selection = current.to_string();

            egui::ComboBox::from_label(LangMessage::ManifestSource.to_string(config.lang))
                .selected_text(if current == default_url {
                    LangMessage::Default.to_string(config.lang)
                } else {
                    current.to_string()
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut config.selected_version_manifest_url,
                        default_url.clone(),
                        format!(
                            "{} ({})",
                            LangMessage::Default.to_string(config.lang),
                            default_url
                        ),
                    );
                    for url in &config.extra_version_manifest_urls {
                        ui.selectable_value(
                            &mut config.selected_version_manifest_url,
                            url.clone(),
                            url,
                        );
                    }
                });

            let after_selection = config.get_effective_version_manifest_url();
            if before_selection != after_selection {
                config.save();
                manifest_state.retry_fetch(runtime, config, ctx);
            }
        }

        if !config.extra_version_manifest_urls.is_empty() {
            ui.separator();
            ui.label(LangMessage::CustomManifests.to_string(config.lang));
            let mut to_remove: Option<String> = None;
            for url in &config.extra_version_manifest_urls {
                ui.horizontal(|ui| {
                    ui.label(url);
                    let in_use = instance_storage.count_instances_with_manifest_url(url) > 0;
                    let delete_enabled = !in_use;
                    if ui
                        .add_enabled(delete_enabled, egui::Button::new("🗑"))
                        .clicked()
                    {
                        to_remove = Some(url.clone());
                    }
                });
            }
            if let Some(url) = to_remove {
                let before = config.get_effective_version_manifest_url().to_string();
                config.remove_version_manifest_url(&url);
                let after = config.get_effective_version_manifest_url();
                if before != after {
                    manifest_state.retry_fetch(runtime, config, ctx);
                }
            }
        }
    }

    fn check_manifest_url(url: &str, config: &Config) -> bool {
        let trimmed = url.trim();
        let is_http = trimmed.starts_with("http://") || trimmed.starts_with("https://");
        let not_default = trimmed != build_config::get_default_version_manifest_url();
        let not_duplicate = !config
            .extra_version_manifest_urls
            .iter()
            .any(|u| u == trimmed);
        is_http && not_default && not_duplicate
    }

    fn render_add_manifest_window(&mut self, ui: &mut egui::Ui, config: &mut Config) {
        if !self.add_manifest_opened {
            return;
        }
        let mut open = true;
        egui::Window::new(LangMessage::AddManifestUrl.to_string(config.lang))
            .open(&mut open)
            .show(ui.ctx(), |ui| {
                ui.label(LangMessage::EnterManifestUrl.to_string(config.lang));
                ui.text_edit_singleline(&mut self.new_manifest_url);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            Self::check_manifest_url(&self.new_manifest_url, config),
                            egui::Button::new(LangMessage::Add.to_string(config.lang)),
                        )
                        .clicked()
                    {
                        config.add_version_manifest_url(self.new_manifest_url.clone());
                        self.new_manifest_url.clear();
                        self.add_manifest_opened = false;
                    }
                    if ui
                        .button(LangMessage::Cancel.to_string(config.lang))
                        .clicked()
                    {
                        self.add_manifest_opened = false;
                    }
                });
            });
        if !open {
            self.add_manifest_opened = false;
        }
    }

    pub fn render_instance_settings(
        &mut self,
        ui: &mut egui::Ui,
        runtime: &Runtime,
        config: &mut Config,
        selected_metadata: Option<&CompleteVersionMetadata>,
    ) {
        if ui
            .add_enabled(
                selected_metadata.is_some() && !self.instance_settings_opened,
                egui::Button::new("⚙"),
            )
            .clicked()
        {
            self.instance_settings_opened = true;
            let selected_metadata = selected_metadata.unwrap();
            self.picked_java_path = config.java_paths.get(selected_metadata.get_name()).cloned();
            self.xmx_slider_value = map_xmx_slider_value_reverse(
                config
                    .xmx
                    .get(selected_metadata.get_name())
                    .unwrap_or(&XMX_DEFAULT.to_string()),
            );
            self.use_native_glfw = *config
                .use_native_glfw
                .get(selected_metadata.get_name())
                .unwrap_or(&USE_NATIVE_GLFW_DEFAULT);
        }

        if let Some(selected_metadata) = selected_metadata {
            self.render_instance_settings_window(ui, runtime, config, selected_metadata);
        } else {
            self.instance_settings_opened = false;
        }
    }

    #[cfg(target_os = "linux")]
    fn render_use_native_glfw_checkbox(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut Config,
        selected_metadata: &CompleteVersionMetadata,
    ) {
        let old_use_native_glfw = self.use_native_glfw;
        ui.checkbox(
            &mut self.use_native_glfw,
            LangMessage::UseNativeGlfw.to_string(config.lang),
        );
        if old_use_native_glfw != self.use_native_glfw {
            config.use_native_glfw.insert(
                selected_metadata.get_name().to_string(),
                self.use_native_glfw,
            );
            config.save();
        }
    }

    fn render_instance_settings_window(
        &mut self,
        ui: &mut egui::Ui,
        runtime: &Runtime,
        config: &mut Config,
        selected_metadata: &CompleteVersionMetadata,
    ) {
        let lang = config.lang;
        let mut settings_opened = self.instance_settings_opened;

        egui::Window::new(LangMessage::InstanceSettings.to_string(lang))
            .open(&mut settings_opened)
            .show(ui.ctx(), |ui| {
                if let Some(picked_java_path) = &self.picked_java_path {
                    ui.label(LangMessage::SelectedJavaPath.to_string(lang));
                    ui.code(picked_java_path);
                } else {
                    ui.label(LangMessage::NoJavaPath.to_string(lang));
                }

                if ui
                    .button(LangMessage::SelectJavaPath.to_string(lang))
                    .clicked()
                    && let Some(path) = rfd::FileDialog::new().pick_file()
                {
                    if runtime.block_on(java::check_java(
                        &selected_metadata.get_java_version(),
                        &path,
                    )) {
                        self.picked_java_path = Some(path.display().to_string());
                        config.java_paths.insert(
                            selected_metadata.get_name().to_string(),
                            path.display().to_string(),
                        );
                        config.save();
                    } else {
                        self.picked_java_path =
                            LangMessage::InvalidJavaInstallation.to_string(lang).into();
                    }
                }

                ui.label(LangMessage::AllocatedMemory.to_string(lang));
                let old_xmx = self.xmx_slider_value;
                let xmx_slider = egui::Slider::new(&mut self.xmx_slider_value, 0.0..=1.0)
                    .custom_formatter(|value, _| {
                        format!(
                            "{}M",
                            ((utils::map_range(value, 0.0, 1.0, XMX_MIN as f64, get_xmx_max())
                                as u64
                                + XMX_STEP / 2)
                                / XMX_STEP)
                                * XMX_STEP
                        )
                    })
                    .custom_parser(|value| {
                        let mb = utils::format_xmx(Some(value))
                            .trim_end_matches('M')
                            .parse::<u64>()
                            .unwrap_or(XMX_DEFAULT);
                        let value_unclamped =
                            utils::map_range(mb as f64, XMX_MIN as f64, get_xmx_max(), 0.0, 1.0);
                        Some(value_unclamped.clamp(0.0, 1.0))
                    });
                ui.add(xmx_slider);
                if old_xmx != self.xmx_slider_value {
                    config.xmx.insert(
                        selected_metadata.get_name().to_string(),
                        map_xmx_slider_value(self.xmx_slider_value),
                    );
                    config.save();
                }

                #[cfg(target_os = "linux")]
                self.render_use_native_glfw_checkbox(ui, config, selected_metadata);
            });

        self.instance_settings_opened = settings_opened;
    }

    fn render_close_launcher_checkbox(&mut self, ui: &mut egui::Ui, config: &mut Config) {
        let old_close_launcher_after_launch = config.hide_launcher_after_launch;
        ui.checkbox(
            &mut config.hide_launcher_after_launch,
            LangMessage::HideLauncherAfterLaunch.to_string(config.lang),
        );
        if old_close_launcher_after_launch != config.hide_launcher_after_launch {
            config.save();
        }
    }
}
