use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use log::error;
use maplit::hashmap;
use shared::generate::extra::ExtraMetadataGenerator;
use shared::generate::manifest::get_version_info;
use shared::loader_generator::fabric::{FabricGenerator, FabricVersionsMeta};
use shared::loader_generator::forge::{
    ForgeGenerator, ForgeMavenMetadata, ForgePromotions, Loader, NeoforgeMavenMetadata,
};
use shared::loader_generator::generator::VersionGenerator;
use shared::loader_generator::vanilla::VanillaGenerator;
use shared::paths::get_minecraft_dir;
use shared::progress::NoProgressBar;
use shared::utils::{VANILLA_MANIFEST_URL, get_vanilla_version_info, is_connect_error};
use shared::version::version_manifest::{VersionInfo, VersionManifest};
use tokio::runtime::Runtime;

use crate::{
    config::runtime_config::Config,
    lang::{Lang, LangMessage},
};

use super::background_task::{BackgroundTask, BackgroundTaskResult};

struct AllVersionsMetadata {
    vanilla_manifest: VersionManifest,
    forge_metadata: ForgeMavenMetadata,
    forge_promotions: ForgePromotions,
    neoforge_metadata: NeoforgeMavenMetadata,
}

fn fetch_all_metadata(
    runtime: &Runtime,
    ctx: &egui::Context,
) -> BackgroundTask<anyhow::Result<AllVersionsMetadata>> {
    let fut = async {
        let result = futures::try_join!(
            VersionManifest::fetch(VANILLA_MANIFEST_URL),
            ForgeMavenMetadata::fetch(),
            ForgePromotions::fetch(),
            NeoforgeMavenMetadata::fetch(),
        );
        let (vanilla_manifest, forge_metadata, forge_promotions, neoforge_metadata) = result?;
        anyhow::Result::Ok(AllVersionsMetadata {
            vanilla_manifest,
            forge_metadata,
            forge_promotions,
            neoforge_metadata,
        })
    };

    let ctx = ctx.clone();
    BackgroundTask::with_callback(
        fut,
        runtime,
        Box::new(move || {
            ctx.request_repaint();
        }),
    )
}

struct PerVersionMetadata {
    fabric_metadata: FabricVersionsMeta,
}

fn fetch_per_version_metadata(
    runtime: &Runtime,
    ctx: &egui::Context,
    version_id: &str,
) -> BackgroundTask<anyhow::Result<PerVersionMetadata>> {
    let version_id = version_id.to_string();
    let fut = async move {
        return anyhow::Result::Ok(PerVersionMetadata {
            fabric_metadata: FabricVersionsMeta::fetch(&version_id).await?,
        });
    };

    let ctx = ctx.clone();
    BackgroundTask::with_callback(
        fut,
        runtime,
        Box::new(move || {
            ctx.request_repaint();
        }),
    )
}

enum NewInstanceMetadataState<MetadataType> {
    NotFetched,
    Fetched(MetadataType),
    OfflineError,
    UnknownError,
}

impl<MetadataType> NewInstanceMetadataState<MetadataType>
where
    MetadataType: Send,
{
    fn take_from_task(&mut self, task: BackgroundTask<anyhow::Result<MetadataType>>) {
        match task.take_result() {
            BackgroundTaskResult::Finished(result) => {
                *self = match result {
                    Ok(metadata) => NewInstanceMetadataState::Fetched(metadata),
                    Err(e) => {
                        if is_connect_error(&e) {
                            NewInstanceMetadataState::OfflineError
                        } else {
                            error!("Error getting metadata:\n{e:?}");
                            NewInstanceMetadataState::UnknownError
                        }
                    }
                };
            }
            BackgroundTaskResult::Cancelled => {
                *self = NewInstanceMetadataState::NotFetched;
            }
        }
    }

    fn render_ui(&self, ui: &mut egui::Ui, lang: Lang, have_task: bool) -> bool {
        match self {
            NewInstanceMetadataState::NotFetched => {
                if have_task {
                    ui.label(LangMessage::LoadingMetadata.to_string(lang));
                }
            }
            NewInstanceMetadataState::OfflineError => {
                ui.label(LangMessage::MetadataErrorOffline.to_string(lang));
            }
            NewInstanceMetadataState::UnknownError => {
                ui.label(LangMessage::MetadataFetchError.to_string(lang));
            }
            NewInstanceMetadataState::Fetched(_) => {}
        }

        if matches!(
            self,
            NewInstanceMetadataState::UnknownError | NewInstanceMetadataState::OfflineError
        ) && ui.button(LangMessage::Retry.to_string(lang)).clicked()
        {
            return true;
        }

        false
    }
}

pub struct RenderUIResult {
    pub instance_to_delete: Option<String>,
}

const VANILLA_LOADER: &str = "Vanilla";
const FABRIC_LOADER: &str = "Fabric";
const FORGE_LOADER: &str = "Forge";
const NEOFORGE_LOADER: &str = "NeoForge";

struct NewInstanceParams {
    instance_name: String,
    minecraft_version: String,
    loader: String,
    loader_version: String,
}

fn create_new_instance(
    runtime: &Runtime,
    ctx: &egui::Context,
    launcher_dir: &Path,
    version_manifest: &VersionManifest,
    new_instance_params: NewInstanceParams,
) -> BackgroundTask<anyhow::Result<VersionInfo>> {
    let NewInstanceParams {
        instance_name,
        minecraft_version,
        loader,
        loader_version,
    } = new_instance_params;

    let launcher_dir = launcher_dir.to_path_buf();
    let version_manifest = version_manifest.clone();
    let fut = async move {
        let vanilla_info = get_vanilla_version_info(&version_manifest, &minecraft_version)?;

        let generator: Box<dyn VersionGenerator + Send> = match loader.as_str() {
            VANILLA_LOADER => {
                if !loader_version.is_empty() {
                    log::warn!("Ignoring loader version for vanilla version");
                }

                Box::new(VanillaGenerator::new(
                    instance_name.to_string(),
                    vanilla_info,
                ))
            }

            FABRIC_LOADER => Box::new(FabricGenerator::new(
                instance_name.to_string(),
                vanilla_info,
                Some(loader_version),
            )),

            FORGE_LOADER => Box::new(ForgeGenerator::new(
                instance_name.to_string(),
                vanilla_info,
                Loader::Forge,
                Some(loader_version),
                Arc::new(NoProgressBar),
            )),

            NEOFORGE_LOADER => Box::new(ForgeGenerator::new(
                instance_name.to_string(),
                vanilla_info,
                Loader::Neoforge,
                Some(loader_version),
                Arc::new(NoProgressBar),
            )),

            _ => {
                return Err(anyhow::Error::msg("Unknown loader"));
            }
        };

        let generator_result = generator.generate(&launcher_dir).await?;

        let extra_generator = ExtraMetadataGenerator::new(
            instance_name.to_string(),
            None,
            generator_result.extra_libs_paths,
            None,
            None,
        );
        let _ = extra_generator.generate(&launcher_dir).await?;

        let version_info = get_version_info(
            &launcher_dir,
            &generator_result.metadata,
            &instance_name,
            None,
        )
        .await?;

        Ok(version_info)
    };

    let ctx = ctx.clone();
    BackgroundTask::with_callback(
        fut,
        runtime,
        Box::new(move || {
            ctx.request_repaint();
        }),
    )
}

enum NewInstanceGenerateState {
    NoError,
    Offline,
    UnknownError,
}

pub struct NewInstanceState {
    window_open: bool,
    new_instance_name: String,
    instance_version: String,
    instance_loader: String,
    instance_loader_version: String,

    instance_metadata_task: Option<BackgroundTask<anyhow::Result<AllVersionsMetadata>>>,
    all_metadata_state: NewInstanceMetadataState<AllVersionsMetadata>,
    current_version_metadata_task: Option<BackgroundTask<anyhow::Result<PerVersionMetadata>>>,
    curent_metadata_state: NewInstanceMetadataState<PerVersionMetadata>,

    instance_generate_task: Option<BackgroundTask<anyhow::Result<VersionInfo>>>,
    instance_generate_state: NewInstanceGenerateState,
    delete_window_open: bool,
    selected_instance_to_delete: String,
    confirm_delete: bool,
}

impl NewInstanceState {
    pub fn new(runtime: &Runtime, ctx: &egui::Context) -> Self {
        Self {
            window_open: false,
            new_instance_name: String::new(),
            instance_version: String::new(),
            instance_loader: String::new(),
            instance_loader_version: String::new(),

            instance_metadata_task: Some(fetch_all_metadata(runtime, ctx)),
            all_metadata_state: NewInstanceMetadataState::NotFetched,
            current_version_metadata_task: None,
            curent_metadata_state: NewInstanceMetadataState::NotFetched,

            instance_generate_task: None,
            instance_generate_state: NewInstanceGenerateState::NoError,
            delete_window_open: false,
            selected_instance_to_delete: String::new(),
            confirm_delete: false,
        }
    }

    pub fn take_new_instance(&mut self) -> Option<VersionInfo> {
        if let Some(task) = self.instance_generate_task.as_ref()
            && task.has_result()
        {
            let task = self.instance_generate_task.take().unwrap();
            match task.take_result() {
                BackgroundTaskResult::Finished(result) => match result {
                    Ok(version_info) => {
                        self.window_open = false;
                        self.instance_generate_state = NewInstanceGenerateState::NoError;
                        return Some(version_info);
                    }
                    Err(e) => {
                        error!("Error creating instance:\n{e:?}");
                        self.instance_generate_state = if is_connect_error(&e) {
                            NewInstanceGenerateState::Offline
                        } else {
                            NewInstanceGenerateState::UnknownError
                        };
                    }
                },
                BackgroundTaskResult::Cancelled => {
                    self.instance_generate_state = NewInstanceGenerateState::NoError;
                }
            }
        }
        None
    }

    pub fn render_ui(
        &mut self,
        runtime: &Runtime,
        ui: &mut egui::Ui,
        config: &mut Config,
        existing_names: &HashSet<String>,
        local_instance_names: &Vec<String>,
    ) -> RenderUIResult {
        let lang = config.lang;

        if let Some(task) = self.instance_metadata_task.as_ref()
            && task.has_result()
        {
            let task = self.instance_metadata_task.take();
            self.all_metadata_state.take_from_task(task.unwrap());
        }
        if let Some(task) = self.current_version_metadata_task.as_mut()
            && task.has_result()
        {
            let task = self.current_version_metadata_task.take();
            self.curent_metadata_state.take_from_task(task.unwrap());
            self.instance_loader = VANILLA_LOADER.to_string();
        }

        if let Some(selected_instance_name) = &config.selected_instance_name
            && ui.button("📂").clicked()
        {
            let launcher_dir = config.get_launcher_dir();
            let _ = open::that(get_minecraft_dir(&launcher_dir, selected_instance_name));
        }

        if ui.button("-").clicked() {
            self.delete_window_open = true;
        }
        if ui.button("+").clicked() {
            self.window_open = true;
        }

        let mut new_instance_window_open = self.window_open;
        egui::Window::new(LangMessage::NewInstance.to_string(lang))
            .open(&mut new_instance_window_open)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    ui.label(LangMessage::NewInstanceName.to_string(lang));
                    ui.text_edit_singleline(&mut self.new_instance_name);
                });

                if self.all_metadata_state.render_ui(ui, lang, self.instance_metadata_task.is_some()) {
                    self.instance_metadata_task = Some(fetch_all_metadata(runtime, ui.ctx()));
                }
                let all_metadata = if let NewInstanceMetadataState::Fetched(all_metadata) = &self.all_metadata_state {
                    all_metadata
                } else {
                    return;
                };

                ui.horizontal(|ui| {
                    ui.label(LangMessage::GameVersion.to_string(lang));
                    let versions = all_metadata
                        .vanilla_manifest
                        .versions
                        .iter()
                        .map(|i| i.get_name())
                        .collect::<Vec<_>>();
                    let mut selected_version = self.instance_version.clone();
                    egui::ComboBox::from_id_salt("versions")
                        .selected_text(selected_version.clone())
                        .show_ui(ui, |ui| {
                            for version in versions.iter() {
                                ui.selectable_value(&mut selected_version, version.to_string(), version.to_string());
                            }
                        });

                    if selected_version != self.instance_version {
                        self.instance_version = selected_version;
                        self.current_version_metadata_task = Some(fetch_per_version_metadata(
                            runtime,
                            ui.ctx(),
                            &self.instance_version,
                        ));
                        self.curent_metadata_state = NewInstanceMetadataState::NotFetched;
                        self.instance_loader = VANILLA_LOADER.to_string();
                        self.instance_loader_version = String::new();
                    }
                });

                if self.curent_metadata_state.render_ui(ui, lang, self.current_version_metadata_task.is_some()) {
                    self.current_version_metadata_task = Some(fetch_per_version_metadata(
                        runtime,
                        ui.ctx(),
                        &self.instance_version,
                    ));
                }
                let current_metadata = if let NewInstanceMetadataState::Fetched(metadata) = &self.curent_metadata_state {
                    metadata
                } else {
                    return;
                };

                let versions = hashmap! {
                    FABRIC_LOADER.to_string() => current_metadata.fabric_metadata.get_versions().into_iter().map(|v| v.to_string()).collect(),
                    FORGE_LOADER.to_string() => all_metadata.forge_metadata.get_matching_versions(&self.instance_version),
                    NEOFORGE_LOADER.to_string() => all_metadata.neoforge_metadata.get_matching_versions(&self.instance_version),
                };

                ui.horizontal(|ui| {
                    let mut loaders = vec![VANILLA_LOADER.to_string()];
                    for loader in [FABRIC_LOADER, FORGE_LOADER, NEOFORGE_LOADER] {
                        let versions = versions.get(loader);
                        if let Some(versions) = versions
                            && !versions.is_empty() {
                                loaders.push(loader.to_string());
                            }
                    }

                    let mut new_instance_loader = self.instance_loader.clone();
                    ui.label(LangMessage::Loader.to_string(lang));
                    egui::ComboBox::from_id_salt("loaders")
                        .selected_text(self.instance_loader.clone())
                        .show_ui(ui, |ui| {
                            for loader in loaders.iter() {
                                ui.selectable_value(&mut new_instance_loader, loader.to_string(), loader.to_string());
                            }
                        });
                    if new_instance_loader != self.instance_loader {
                        self.instance_loader = new_instance_loader;
                        self.instance_loader_version = String::new();
                    }
                });

                if let Some(versions) = versions.get(&self.instance_loader) {
                    ui.horizontal(|ui| {
                        let mut version_name = HashMap::new();
                        for version in versions.iter() {
                            version_name.insert(version.to_string(), version.to_string());
                        }
                        let latest_types = ["recommended", "latest"];
                        if self.instance_loader == FORGE_LOADER {
                            for latest_type in latest_types.iter() {
                                if let Some(promotion) = all_metadata.forge_promotions.get_latest_version(&self.instance_version, latest_type) {
                                    version_name.insert(promotion.to_string(), format!("{promotion} ({latest_type})"));
                                }
                            }
                        }

                        if self.instance_loader_version.is_empty() && self.instance_loader == FORGE_LOADER {
                            for latest_type in latest_types.iter() {
                                if let Some(promotion) = all_metadata.forge_promotions.get_latest_version(&self.instance_version, latest_type) {
                                    self.instance_loader_version = promotion.to_string();
                                    break;
                                }
                            }
                        }
                        if self.instance_loader_version.is_empty()
                            && let Some(version) = versions.first() {
                                self.instance_loader_version = version.to_string();
                            }

                        ui.label(LangMessage::LoaderVersion.to_string(lang));
                        egui::ComboBox::from_id_salt("loader_versions")
                            .selected_text(self.instance_loader_version.clone())
                            .show_ui(ui, |ui| {
                                for version in versions.iter() {
                                    ui.selectable_value(&mut self.instance_loader_version, version.to_string(), version_name.get(version).unwrap_or(version).to_string());
                                }
                            });
                    });
                }

                if !self.new_instance_name.is_empty() && (self.instance_loader == VANILLA_LOADER || versions.contains_key(&self.instance_loader)) {
                    if existing_names.contains(&self.new_instance_name) {
                        ui.label(LangMessage::InstanceNameExists.to_string(lang));
                    } else {
                        ui.horizontal(|ui| {
                            if self.instance_generate_task.is_none() {
                                if ui.button(LangMessage::CreateInstance.to_string(lang)).clicked() {
                                    let params = NewInstanceParams {
                                        instance_name: self.new_instance_name.clone(),
                                        minecraft_version: self.instance_version.clone(),
                                        loader: self.instance_loader.clone(),
                                        loader_version: self.instance_loader_version.clone(),
                                    };
                                    let task = create_new_instance(
                                        runtime,
                                        ui.ctx(),
                                        &config.get_launcher_dir(),
                                        &all_metadata.vanilla_manifest,
                                        params,
                                    );
                                    self.instance_generate_task = Some(task);
                                }
                                match self.instance_generate_state {
                                    NewInstanceGenerateState::Offline => {
                                        ui.label(LangMessage::InstanceGenerateErrorOffline.to_string(lang));
                                    }
                                    NewInstanceGenerateState::UnknownError => {
                                        ui.label(LangMessage::InstanceGenerateError.to_string(lang));
                                    }
                                    _ => {}
                                }
                            } else {
                                ui.label(LangMessage::CreatingInstance.to_string(lang));
                                if ui.button(LangMessage::Cancel.to_string(lang)).clicked() {
                                    self.instance_generate_task = None;
                                }
                            }
                        });
                        if self.instance_generate_task.is_some() && [FORGE_LOADER, NEOFORGE_LOADER].contains(&self.instance_loader.as_str()) {
                            ui.label(LangMessage::LongTimeWarning.to_string(lang));
                        }
                    }
                }
            });
        self.window_open = new_instance_window_open;

        let mut delete_window_open = self.delete_window_open;
        let mut close_delete_window = false;
        let mut instance_to_delete = None;
        egui::Window::new(LangMessage::DeleteInstance.to_string(lang))
            .open(&mut delete_window_open)
            .show(ui.ctx(), |ui| {
                ui.label(LangMessage::SelectInstanceToDelete.to_string(lang));
                egui::ComboBox::from_id_salt("delete_instances")
                    .selected_text(if self.selected_instance_to_delete.is_empty() {
                        LangMessage::NotSelected.to_string(lang)
                    } else {
                        self.selected_instance_to_delete.clone()
                    })
                    .show_ui(ui, |ui| {
                        for instance_name in local_instance_names {
                            ui.selectable_value(
                                &mut self.selected_instance_to_delete,
                                instance_name.clone(),
                                instance_name,
                            );
                        }
                    });

                ui.checkbox(
                    &mut self.confirm_delete,
                    LangMessage::ConfirmDelete.to_string(lang),
                );

                ui.horizontal(|ui| {
                    let delete_enabled =
                        !self.selected_instance_to_delete.is_empty() && self.confirm_delete;
                    if ui
                        .add_enabled(
                            delete_enabled,
                            egui::Button::new(LangMessage::Delete.to_string(lang)),
                        )
                        .clicked()
                    {
                        instance_to_delete = Some(self.selected_instance_to_delete.clone());
                        self.selected_instance_to_delete.clear();
                        self.confirm_delete = false;
                        close_delete_window = true;
                        config.selected_instance_name = None;
                    }
                });
            });
        if close_delete_window {
            self.delete_window_open = false;
        } else {
            self.delete_window_open = delete_window_open;
        }

        RenderUIResult { instance_to_delete }
    }
}
