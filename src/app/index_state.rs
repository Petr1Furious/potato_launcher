use std::{path::PathBuf, sync::mpsc};

use crate::{
    config::runtime_config,
    lang::LangMessage,
    modpack::index::{load_local_indexes, load_remote_indexes, ModpackIndex},
};

use super::task::Task;

#[derive(Clone, PartialEq)]
enum FetchStatus {
    Fetching,
    FetchedRemote,
    FetchedLocalRemoteError(String),
    FetchedLocalOffline,
}

struct IndexFetchResult {
    status: FetchStatus,
    indexes: Vec<ModpackIndex>,
}

fn fetch_indexes<Callback>(
    runtime: &tokio::runtime::Runtime,
    index_path: PathBuf,
    callback: Callback,
) -> Task<IndexFetchResult>
where
    Callback: FnOnce() + Send + 'static,
{
    let (tx, rx) = mpsc::channel();

    runtime.spawn(async move {
        let indexes = match load_remote_indexes().await {
            Ok(i) => IndexFetchResult {
                status: FetchStatus::FetchedRemote,
                indexes: i,
            },
            Err(e) => {
                let mut connect_error = false;
                if let Some(re) = e.downcast_ref::<reqwest::Error>() {
                    if re.is_connect() {
                        connect_error = true;
                    }
                }

                IndexFetchResult {
                    status: if connect_error {
                        FetchStatus::FetchedLocalOffline
                    } else {
                        FetchStatus::FetchedLocalRemoteError(e.to_string())
                    },
                    indexes: load_local_indexes(&index_path),
                }
            }
        };

        let _ = tx.send(indexes);
        callback();
    });

    return Task::new(rx);
}

pub struct IndexState {
    status: FetchStatus,
    fetch_task: Option<Task<IndexFetchResult>>,
    indexes: Option<Vec<ModpackIndex>>,
}

#[derive(PartialEq)]
pub enum UpdateResult {
    IndexesNotUpdated,
    IndexesUpdated,
}

impl IndexState {
    pub fn new() -> Self {
        return IndexState {
            status: FetchStatus::Fetching,
            fetch_task: None,
            indexes: None,
        };
    }

    pub fn update(
        &mut self,
        runtime: &tokio::runtime::Runtime,
        config: &mut runtime_config::Config,
        ctx: &egui::Context,
    ) -> UpdateResult {
        if self.status == FetchStatus::Fetching && self.fetch_task.is_none() {
            let index_path = runtime_config::get_index_path(config);
            let ctx = ctx.clone();
            self.fetch_task = Some(fetch_indexes(runtime, index_path.clone(), move || {
                ctx.request_repaint();
            }));
        }

        if let Some(task) = self.fetch_task.as_ref() {
            if let Some(result) = task.take_result() {
                self.status = result.status.clone();
                if config.modpack_name.is_none() && result.indexes.len() == 1 {
                    config.modpack_name = result.indexes.first().map(|x| x.modpack_name.clone());
                    runtime_config::save_config(config);
                }
                self.indexes = Some(result.indexes.clone());
                self.fetch_task = None;
                return UpdateResult::IndexesUpdated;
            }
        }
        UpdateResult::IndexesNotUpdated
    }

    pub fn render_ui(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut runtime_config::Config,
    ) -> UpdateResult {
        ui.label(match self.status {
            FetchStatus::Fetching => LangMessage::FetchingModpackIndexes.to_string(&config.lang),
            FetchStatus::FetchedRemote => LangMessage::FetchedRemoteIndexes.to_string(&config.lang),
            FetchStatus::FetchedLocalOffline => {
                LangMessage::NoConnectionToIndexServer.to_string(&config.lang)
            }
            FetchStatus::FetchedLocalRemoteError(ref s) => {
                LangMessage::ErrorFetchingRemoteIndexes(s.clone()).to_string(&config.lang)
            }
        });

        let selected_modpack_name = config.modpack_name.clone();
        let mut just_selected_modpack: Option<&ModpackIndex> = None;

        ui.horizontal(|ui| {
            egui::ComboBox::from_id_source("modpacks")
                .selected_text(
                    selected_modpack_name
                        .unwrap_or_else(|| LangMessage::SelectModpack.to_string(&config.lang)),
                )
                .show_ui(ui, |ui| match self.indexes.as_ref() {
                    Some(r) => {
                        for index in r {
                            ui.selectable_value(
                                &mut just_selected_modpack,
                                Some(index),
                                index.modpack_name.clone(),
                            );
                        }
                    }
                    None => {
                        ui.label(LangMessage::NoIndexes.to_string(&config.lang));
                    }
                });

            if self.status != FetchStatus::FetchedRemote && self.status != FetchStatus::Fetching {
                if ui
                    .button(LangMessage::FetchIndexes.to_string(&config.lang))
                    .clicked()
                {
                    self.status = FetchStatus::Fetching;
                }
            }
        });

        let just_selected_modpack = just_selected_modpack.map(|x| x.clone());
        let just_selected_modpack_name = just_selected_modpack
            .as_ref()
            .map(|x| x.modpack_name.clone());
        if just_selected_modpack != None && config.modpack_name != just_selected_modpack_name {
            config.modpack_name = just_selected_modpack_name;
            runtime_config::save_config(config);
            UpdateResult::IndexesUpdated
        } else {
            UpdateResult::IndexesNotUpdated
        }
    }

    pub fn get_selected_modpack(&self, config: &runtime_config::Config) -> Option<&ModpackIndex> {
        return self.indexes.as_ref().and_then(|indexes| {
            indexes
                .iter()
                .find(|x| Some(&x.modpack_name) == config.modpack_name.as_ref())
        });
    }

    pub fn online(&self) -> bool {
        self.status == FetchStatus::FetchedRemote
    }
}
