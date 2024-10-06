use std::{
    path::Path,
    sync::{mpsc, Arc},
};

use shared::version::version_manifest::VersionInfo;
use tokio_util::sync::CancellationToken;

use crate::{
    config::runtime_config,
    lang::LangMessage,
    version::complete_version_metadata::{
        get_complete_version_metadata, read_local_complete_version_metadata,
        CompleteVersionMetadata,
    },
};

use super::task::Task;

#[derive(PartialEq)]
enum GetStatus {
    Getting,
    UpToDate,
    ReadLocalRemoteError(String),
    ReadLocalOffline,
    ErrorGetting(String),
}

struct MetadataFetchResult {
    status: GetStatus,
    metadata: Option<CompleteVersionMetadata>,
}

fn get_metadata<Callback>(
    runtime: &tokio::runtime::Runtime,
    version_info: &VersionInfo,
    data_dir: &Path,
    cancellation_token: CancellationToken,
    callback: Callback,
) -> Task<MetadataFetchResult>
where
    Callback: FnOnce() + Send + 'static,
{
    let (tx, rx) = mpsc::channel();

    let version_info = version_info.clone();
    let data_dir = data_dir.to_path_buf();

    runtime.spawn(async move {
        let fut = get_complete_version_metadata(&version_info, &data_dir);
        let metadata = tokio::select! {
            _ = cancellation_token.cancelled() => {
                MetadataFetchResult {
                    status: GetStatus::Getting,
                    metadata: None,
                }
            }
            result = fut => match result {
                Ok(metadata) => MetadataFetchResult {
                    status: GetStatus::UpToDate,
                    metadata: Some(metadata),
                },
                Err(e) => {
                    let mut connect_error = false;
                    if let Some(re) = e.downcast_ref::<reqwest::Error>() {
                        if re.is_connect() {
                            connect_error = true;
                        }
                    }

                    let local_metadata = read_local_complete_version_metadata(&version_info, &data_dir).await;
                    MetadataFetchResult {
                        status: if connect_error {
                            GetStatus::ReadLocalOffline
                        } else if local_metadata.is_err() {
                            GetStatus::ErrorGetting(e.to_string())
                        } else {
                            GetStatus::ReadLocalRemoteError(e.to_string())
                        },
                        metadata: local_metadata.ok(),
                    }
                }
            }
        };

        let _ = tx.send(metadata);
        callback();
    });

    return Task::new(rx);
}

pub struct MetadataState {
    status: GetStatus,
    get_task: Option<Task<MetadataFetchResult>>,
    metadata: Option<Arc<CompleteVersionMetadata>>,
    cancellation_token: CancellationToken,
}

pub enum UpdateResult {
    MetadataNotUpdated,
    MetadataUpdated,
}

impl MetadataState {
    pub fn new() -> Self {
        return MetadataState {
            status: GetStatus::Getting,
            get_task: None,
            metadata: None,
            cancellation_token: CancellationToken::new(),
        };
    }

    pub fn reset(&mut self) {
        self.status = GetStatus::Getting;
        self.get_task = None;
        self.metadata = None;
        self.cancellation_token.cancel();
        self.cancellation_token = CancellationToken::new();
    }

    pub fn update(
        &mut self,
        runtime: &tokio::runtime::Runtime,
        config: &mut runtime_config::Config,
        version_info: &VersionInfo,
        ctx: &egui::Context,
    ) -> UpdateResult {
        if self.status == GetStatus::Getting && self.get_task.is_none() {
            let launcher_dir = runtime_config::get_launcher_dir(config);

            let ctx = ctx.clone();
            self.cancellation_token = CancellationToken::new();

            self.get_task = Some(get_metadata(
                runtime,
                version_info,
                &launcher_dir,
                self.cancellation_token.clone(),
                move || {
                    ctx.request_repaint();
                },
            ));
        }

        if let Some(task) = self.get_task.as_ref() {
            if let Some(result) = task.take_result() {
                self.status = result.status;
                self.metadata = result.metadata.map(Arc::new);
                self.get_task = None;
                return UpdateResult::MetadataUpdated;
            }
        }

        UpdateResult::MetadataNotUpdated
    }

    pub fn render_ui(&mut self, ui: &mut egui::Ui, config: &runtime_config::Config) {
        match self.status {
            GetStatus::Getting => {
                ui.label(LangMessage::GettingVersionMetadata.to_string(&config.lang));
            }
            GetStatus::ReadLocalOffline => {
                ui.label(LangMessage::NoConnectionToMetadataServer.to_string(&config.lang));
                self.render_retry_button(ui, config);
            }
            GetStatus::ReadLocalRemoteError(ref s) => {
                ui.label(
                    LangMessage::ErrorGettingRemoteMetadata(s.clone()).to_string(&config.lang),
                );
                self.render_retry_button(ui, config);
            }
            GetStatus::ErrorGetting(ref s) => {
                ui.label(LangMessage::ErrorGettingMetadata(s.clone()).to_string(&config.lang));
                self.render_retry_button(ui, config);
            }
            GetStatus::UpToDate => {}
        }
    }

    fn render_retry_button(&mut self, ui: &mut egui::Ui, config: &runtime_config::Config) {
        if ui
            .button(LangMessage::Retry.to_string(&config.lang))
            .clicked()
        {
            self.reset();
        }
    }

    pub fn get_version_metadata(&self) -> Option<Arc<CompleteVersionMetadata>> {
        self.metadata.clone()
    }

    pub fn online(&self) -> bool {
        self.status == GetStatus::UpToDate
    }
}
