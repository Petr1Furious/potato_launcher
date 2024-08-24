use crate::{lang::LangMessage, progress::ProgressBar};
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
}

impl GuiProgressBar {
    pub fn new(ctx: &egui::Context) -> Self {
        Self {
            state: Arc::new(Mutex::new(ProgressBarState {
                progress: 0,
                total: 0,
                message: None,
                finished: false,
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
}
