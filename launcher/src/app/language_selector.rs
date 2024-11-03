use crate::{
    config::runtime_config,
    lang::{Lang, LangMessage},
};

pub struct LanguageSelector {}

impl LanguageSelector {
    pub fn new() -> Self {
        LanguageSelector {}
    }

    pub fn render_ui(&mut self, ui: &mut egui::Ui, config: &mut runtime_config::Config) {
        let mut lang = config.lang.clone();
        egui::ComboBox::from_id_source("language_selector")
            .selected_text("🌐 ".to_string() + &LangMessage::LanguageName.to_string(config.lang))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut lang,
                    Lang::English,
                    LangMessage::LanguageName.to_string(Lang::English),
                );
                ui.selectable_value(
                    &mut lang,
                    Lang::Russian,
                    LangMessage::LanguageName.to_string(Lang::Russian),
                );
            });
        if lang != config.lang {
            config.lang = lang;
            config.save();
        }
    }
}
