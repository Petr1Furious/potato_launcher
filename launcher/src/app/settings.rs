use crate::config::runtime_config::Config;
use crate::lang::LangMessage;
use crate::utils;
use crate::version::complete_version_metadata::CompleteVersionMetadata;

use shared::java;
use tokio::runtime::Runtime;

use super::language_selector::LanguageSelector;

pub struct SettingsState {
    language_selector: LanguageSelector,
    settings_opened: bool,
    picked_java_path: Option<String>,
    selected_xmx: Option<String>,
}

impl SettingsState {
    pub fn new() -> Self {
        Self {
            language_selector: LanguageSelector::new(),
            settings_opened: false,
            picked_java_path: None,
            selected_xmx: None,
        }
    }
    pub fn render_ui(
        &mut self,
        ui: &mut egui::Ui,
        runtime: &Runtime,
        config: &mut Config,
        selected_metadata: Option<&CompleteVersionMetadata>,
    ) {
        if ui.button("⚙").clicked() {
            self.settings_opened = true;

            self.picked_java_path = if let Some(selected_metadata) = selected_metadata {
                config
                    .java_paths
                    .get(&selected_metadata.get_name().to_string())
                    .cloned()
            } else {
                None
            };
            self.selected_xmx = Some(config.xmx.clone());
        }

        if ui.button("📂").clicked() {
            open::that(config.get_launcher_dir()).unwrap();
        }

        self.language_selector.render_ui(ui, config);

        self.render_settings_window(ui, runtime, config, selected_metadata);
    }

    fn render_settings_window(
        &mut self,
        ui: &mut egui::Ui,
        runtime: &Runtime,
        config: &mut Config,
        selected_metadata: Option<&CompleteVersionMetadata>,
    ) {
        let mut settings_opened = self.settings_opened.clone();

        let mut update_status = false;
        egui::Window::new(LangMessage::Settings.to_string(config.lang))
            .open(&mut settings_opened)
            .show(ui.ctx(), |ui| {
                if let Some(picked_java_path) = &self.picked_java_path {
                    ui.label(LangMessage::SelectedJavaPath.to_string(config.lang));
                    ui.code(picked_java_path);
                } else {
                    ui.label(LangMessage::NoJavaPath.to_string(config.lang));
                }

                let button = egui::Button::new(LangMessage::SelectJavaPath.to_string(config.lang));
                if ui
                    .add_enabled(selected_metadata.is_some(), button)
                    .clicked()
                {
                    if let Some(selected_metadata) = selected_metadata {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
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
                                update_status = true;
                            } else {
                                self.picked_java_path =
                                    "Invalid Java Installation".to_string().into();
                            }
                        }
                    }
                }

                ui.label(LangMessage::JavaXMX.to_string(config.lang));
                ui.text_edit_singleline(self.selected_xmx.as_mut().unwrap());

                if utils::validate_xmx(self.selected_xmx.as_ref().unwrap())
                    && config.xmx != self.selected_xmx.as_ref().unwrap().as_str()
                {
                    config.xmx = self.selected_xmx.as_ref().unwrap().clone();
                    config.save();
                }

                self.render_close_launcher_checkbox(ui, config);
            });

        self.settings_opened = settings_opened;
    }

    fn render_close_launcher_checkbox(&mut self, ui: &mut egui::Ui, config: &mut Config) {
        let old_close_launcher_after_launch = config.close_launcher_after_launch;
        ui.checkbox(
            &mut config.close_launcher_after_launch,
            LangMessage::CloseLauncherAfterLaunch.to_string(config.lang),
        );
        if old_close_launcher_after_launch != config.close_launcher_after_launch {
            config.save();
        }
    }
}
