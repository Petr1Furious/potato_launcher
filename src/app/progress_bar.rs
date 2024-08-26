use egui::Widget as _;

use crate::{
    lang::{Lang, LangMessage},
    progress::{ProgressBar, Unit},
};
use std::sync::{Arc, Mutex};

pub struct GuiProgressBar {
    state: Arc<Mutex<ProgressBarState>>,
    ctx: egui::Context,
    last_update: Arc<Mutex<std::time::Instant>>,
}

#[derive(Clone)]
pub struct ProgressBarState {
    pub progress: u64,
    pub total: u64,
    pub message: Option<LangMessage>,
    pub finished: bool,
    pub unit: Option<Unit>,
}

impl GuiProgressBar {
    pub fn new(ctx: &egui::Context) -> Self {
        Self {
            state: Arc::new(Mutex::new(ProgressBarState {
                progress: 0,
                total: 0,
                message: None,
                finished: false,
                unit: None,
            })),
            ctx: ctx.clone(),
            last_update: Arc::new(Mutex::new(std::time::Instant::now())),
        }
    }

    pub fn get_state(&self) -> ProgressBarState {
        self.state.lock().unwrap().clone()
    }

    fn update_gui_if_needed(&self) {
        let now = std::time::Instant::now();
        let mut last_update = self.last_update.lock().unwrap();
        if now.duration_since(*last_update).as_millis() > 10 {
            self.ctx.request_repaint();
            *last_update = now;
        }
    }
}

impl ProgressBar for GuiProgressBar {
    fn set_message(&self, message: LangMessage) {
        let mut state = self.state.lock().unwrap();
        state.message = Some(message);
    }

    fn set_length(&self, length: u64) {
        let mut state = self.state.lock().unwrap();
        state.total = length;
        state.progress = 0;
        state.finished = false;
    }

    fn inc(&self, amount: u64) {
        let mut state = self.state.lock().unwrap();
        state.progress += amount;
        self.update_gui_if_needed();
    }

    fn finish(&self) {
        let mut state = self.state.lock().unwrap();
        state.finished = true;
        self.ctx.request_repaint();
    }

    fn set_unit(&self, unit: Unit) {
        let mut state = self.state.lock().unwrap();
        state.unit = Some(unit);
    }
}

impl GuiProgressBar {
    pub fn render(&self, ui: &mut egui::Ui, lang: &Lang) {
        let progress_bar_state = self.get_state();
        if let Some(message) = &progress_bar_state.message {
            ui.label(message.to_string(lang));
        }

        let unit_size = progress_bar_state
            .unit
            .as_ref()
            .map(|u| u.size as f32)
            .unwrap_or(1.0);
        let unit_name = progress_bar_state.unit.map(|u| u.name);

        let progress_string = if let Some(unit_name) = unit_name {
            let progress = progress_bar_state.progress as f32 / unit_size;
            let total = progress_bar_state.total as f32 / unit_size;
            format!("{:.2} / {:.2} {}", progress, total, unit_name)
        } else {
            format!(
                "{} / {}",
                progress_bar_state.progress, progress_bar_state.total
            )
        };
        egui::ProgressBar::new(
            progress_bar_state.progress as f32 / progress_bar_state.total as f32,
        )
        .text(progress_string)
        .ui(ui);
    }
}
