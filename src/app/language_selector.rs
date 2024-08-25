use crate::{config::runtime_config, lang::{Lang, LangMessage}};

pub struct LanguageSelector {
}

impl LanguageSelector {
    pub fn new() -> Self {
        LanguageSelector {
        }
    }

    pub fn render_ui(&mut self, ui: &mut egui::Ui, config: &mut runtime_config::Config) {
        ui.horizontal(|ui| {
            ui.label(LangMessage::Language.to_string(&config.lang));
            let mut lang = config.lang.clone();
            egui::ComboBox::from_id_source("language_selector")
                .selected_text(LangMessage::LanguageName.to_string(&config.lang))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut lang, Lang::English, LangMessage::LanguageName.to_string(&Lang::English));
                    ui.selectable_value(&mut lang, Lang::Russian, LangMessage::LanguageName.to_string(&Lang::Russian));
                });
            if lang != config.lang {
                config.lang = lang;
                runtime_config::save_config(config);
            }
        });
    }
}
