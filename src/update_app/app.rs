use std::sync::mpsc;
use std::sync::Arc;

use eframe::egui;
use eframe::run_native;
use tokio::runtime::Runtime;

use crate::app::progress_bar::GuiProgressBar;
use crate::config::build_config;
use crate::config::runtime_config;
use crate::lang::Lang;
use crate::lang::LangMessage;
use crate::launcher::update::fetch_new_binary;
use crate::launcher::update::need_update;
use crate::launcher::update::replace_binary_and_launch;
use crate::progress::ProgressBar;
use crate::progress::Unit;

enum UpdateStatus {
    Checking,
    NeedUpdate,
    UpToDate,
    Error(String),
}

enum FetchStatus {
    NeedFetching,
    Error(String),
}

pub struct UpdateApp {
    runtime: Runtime,
    lang: Lang,
    need_update_receiver: mpsc::Receiver<UpdateStatus>,
    new_binary_receiver: Option<mpsc::Receiver<Vec<u8>>>,
    update_progress_bar: Arc<GuiProgressBar>,
    update_status: UpdateStatus,
    fetch_status: FetchStatus,
}

pub fn run_gui(config: &runtime_config::Config) {
    if std::env::var("CARGO").is_ok() {
        println!("Running from cargo, skipping auto-update");
        return;
    }

    if build_config::get_version().is_none() {
        println!("Version not set, skipping auto-update");
        return;
    }

    let native_options = eframe::NativeOptions {
        ..Default::default()
    };

    let lang = config.lang.clone();

    run_native(
        "Launcher",
        native_options,
        Box::new(|cc| Ok(Box::new(UpdateApp::new(lang, &cc.egui_ctx)))),
    )
    .unwrap();
}

impl eframe::App for UpdateApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }
}

impl UpdateApp {
    fn new(lang: Lang, ctx: &egui::Context) -> Self {
        let runtime = Runtime::new().unwrap();

        let (need_update_sender, need_update_receiver) = mpsc::channel();
        runtime.spawn(async move {
            let _ = need_update_sender.send(match need_update().await {
                Ok(true) => UpdateStatus::NeedUpdate,
                Ok(false) => UpdateStatus::UpToDate,
                Err(e) => UpdateStatus::Error(e.to_string()),
            });
        });

        let update_progress_bar = Arc::new(GuiProgressBar::new(ctx));
        update_progress_bar.set_unit(Unit {
            name: "MB".to_string(),
            size: 1024 * 1024,
        });

        UpdateApp {
            runtime,
            lang,
            need_update_receiver,
            new_binary_receiver: None,
            update_progress_bar,
            update_status: UpdateStatus::Checking,
            fetch_status: FetchStatus::NeedFetching,
        }
    }

    fn render_close_button(&self, ui: &mut egui::Ui) {
        if ui
            .button(LangMessage::Close.to_string(&self.lang))
            .clicked()
        {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(new_binary_receiver) = &self.new_binary_receiver {
                match new_binary_receiver.try_recv() {
                    Ok(new_binary) => {
                        ui.label(LangMessage::Launching.to_string(&self.lang));
                        if let Some(e) = replace_binary_and_launch(new_binary.as_slice()).err() {
                            self.fetch_status = FetchStatus::Error(e.to_string());
                        } else {
                            // new binary is already launched
                        }
                    }
                    Err(e) => {
                        self.fetch_status = FetchStatus::Error(e.to_string());
                    }
                }
            } else {
                if let Ok(update_status) = self.need_update_receiver.try_recv() {
                    match &update_status {
                        UpdateStatus::NeedUpdate => {
                            let (new_binary_sender, new_binary_receiver) = mpsc::channel();
                            self.new_binary_receiver = Some(new_binary_receiver);
                            let update_progress_bar = self.update_progress_bar.clone();
                            self.runtime.spawn(async move {
                                let _ = new_binary_sender
                                    .send(fetch_new_binary(update_progress_bar).await.unwrap());
                            });
                        }
                        UpdateStatus::UpToDate => {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        UpdateStatus::Error(_) => {}
                        UpdateStatus::Checking => {}
                    }
                    self.update_status = update_status;
                }
            }

            match &self.update_status {
                UpdateStatus::Checking => {
                    ui.label(LangMessage::CheckingForUpdates.to_string(&self.lang));
                }
                UpdateStatus::NeedUpdate => match &self.fetch_status {
                    FetchStatus::NeedFetching => {
                        self.update_progress_bar.render(ui, &self.lang);
                    }
                    FetchStatus::Error(e) => {
                        ui.label(
                            LangMessage::ErrorDownloadingUpdate(e.to_string())
                                .to_string(&self.lang),
                        );
                        self.render_close_button(ui);
                    }
                },
                UpdateStatus::UpToDate => {}
                UpdateStatus::Error(e) => {
                    ui.label(
                        LangMessage::ErrorCheckingForUpdates(e.to_string()).to_string(&self.lang),
                    );
                    self.render_close_button(ui);
                }
            }
        });
    }
}
