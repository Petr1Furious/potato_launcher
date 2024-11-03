use std::{path::Path, sync::Arc};

use shared::version::version_manifest::VersionInfo;

use crate::{
    config::runtime_config, lang::LangMessage,
    version::complete_version_metadata::CompleteVersionMetadata,
};

use super::background_task::{BackgroundTask, BackgroundTaskResult};

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
    callback: Callback,
) -> BackgroundTask<MetadataFetchResult>
where
    Callback: FnOnce() + Send + 'static,
{
    let version_info = version_info.clone();
    let data_dir = data_dir.to_path_buf();

    let fut = async move {
        let result = CompleteVersionMetadata::read_or_download(&version_info, &data_dir).await;
        match result {
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

                let local_metadata =
                    CompleteVersionMetadata::read_local(&version_info, &data_dir).await;
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

    BackgroundTask::with_callback(fut, runtime, Box::new(callback))
}

pub struct MetadataState {
    status: GetStatus,
    get_task: Option<BackgroundTask<MetadataFetchResult>>,
    metadata: Option<Arc<CompleteVersionMetadata>>,
}

impl MetadataState {
    pub fn new() -> Self {
        return MetadataState {
            status: GetStatus::Getting,
            get_task: None,
            metadata: None,
        };
    }

    pub fn reset(&mut self) {
        self.status = GetStatus::Getting;
        self.get_task = None;
        self.metadata = None;
    }

    pub fn update(
        &mut self,
        runtime: &tokio::runtime::Runtime,
        config: &mut runtime_config::Config,
        version_info: &VersionInfo,
        ctx: &egui::Context,
    ) -> bool {
        if self.status == GetStatus::Getting && self.get_task.is_none() {
            let launcher_dir = config.get_launcher_dir();

            let ctx = ctx.clone();
            self.get_task = Some(get_metadata(
                runtime,
                version_info,
                &launcher_dir,
                move || {
                    ctx.request_repaint();
                },
            ));
        }

        if let Some(task) = self.get_task.as_ref() {
            if task.has_result() {
                let task = self.get_task.take().unwrap();
                let result = task.take_result();
                match result {
                    BackgroundTaskResult::Finished(result) => {
                        self.status = result.status;
                        self.metadata = result.metadata.map(Arc::new);
                    }
                    BackgroundTaskResult::Cancelled => {
                        self.status = GetStatus::Getting;
                    }
                }

                return true;
            }
        }

        false
    }

    pub fn render_ui(&mut self, ui: &mut egui::Ui, config: &runtime_config::Config) -> bool {
        let mut need_separator = true;
        match self.status {
            GetStatus::Getting => {
                ui.label(LangMessage::GettingVersionMetadata.to_string(config.lang));
            }
            GetStatus::ReadLocalOffline => {
                ui.label(LangMessage::NoConnectionToMetadataServer.to_string(config.lang));
                self.render_retry_button(ui, config);
            }
            GetStatus::ReadLocalRemoteError(ref s) => {
                ui.label(LangMessage::ErrorGettingRemoteMetadata(s.clone()).to_string(config.lang));
                self.render_retry_button(ui, config);
            }
            GetStatus::ErrorGetting(ref s) => {
                ui.label(LangMessage::ErrorGettingMetadata(s.clone()).to_string(config.lang));
                self.render_retry_button(ui, config);
            }
            GetStatus::UpToDate => {
                need_separator = false;
            }
        }

        need_separator
    }

    fn render_retry_button(&mut self, ui: &mut egui::Ui, config: &runtime_config::Config) {
        if ui
            .button(LangMessage::Retry.to_string(config.lang))
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
