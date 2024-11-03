use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use maplit::hashmap;
use shared::generate::extra::ExtraMetadataGenerator;
use shared::generate::manifest::get_version_info;
use shared::loader_generator::fabric::{FabricGenerator, FabricVersionsMeta};
use shared::loader_generator::forge::{
    ForgeGenerator, ForgeMavenMetadata, ForgePromotions, Loader, NeoforgeMavenMetadata,
};
use shared::loader_generator::generator::VersionGenerator;
use shared::loader_generator::vanilla::VanillaGenerator;
use shared::paths::get_instance_dir;
use shared::progress::{NoProgressBar, ProgressBar};
use shared::utils::{get_vanilla_version_info, BoxError, BoxResult, VANILLA_MANIFEST_URL};
use shared::version::extra_version_metadata::AuthData;
use shared::version::version_manifest::{VersionInfo, VersionManifest};
use tokio::runtime::Runtime;

use crate::version::complete_version_metadata::CompleteVersionMetadata;
use crate::version::sync::sync_instance;
use crate::{
    config::runtime_config::Config,
    lang::{Lang, LangMessage},
    utils,
};

use super::background_task::{BackgroundTask, BackgroundTaskResult};
use super::progress_bar::GuiProgressBar;

struct AllVersionsMetadata {
    vanilla_manifest: VersionManifest,
    forge_metadata: ForgeMavenMetadata,
    forge_promotions: ForgePromotions,
    neoforge_metadata: NeoforgeMavenMetadata,
}

fn fetch_all_metadata(
    runtime: &Runtime,
    ctx: &egui::Context,
) -> BackgroundTask<BoxResult<AllVersionsMetadata>> {
    let fut = async {
        let result = futures::try_join!(
            VersionManifest::fetch(VANILLA_MANIFEST_URL),
            ForgeMavenMetadata::fetch(),
            ForgePromotions::fetch(),
            NeoforgeMavenMetadata::fetch(),
        );
        let (vanilla_manifest, forge_metadata, forge_promotions, neoforge_metadata) = result?;
        return BoxResult::Ok(AllVersionsMetadata {
            vanilla_manifest,
            forge_metadata,
            forge_promotions,
            neoforge_metadata,
        });
    };

    let ctx = ctx.clone();
    return BackgroundTask::with_callback(
        fut,
        runtime,
        Box::new(move || {
            ctx.request_repaint();
        }),
    );
}

struct PerVersionMetadata {
    fabric_metadata: FabricVersionsMeta,
}

fn fetch_per_version_metadata(
    runtime: &Runtime,
    ctx: &egui::Context,
    version_id: &str,
) -> BackgroundTask<BoxResult<PerVersionMetadata>> {
    let version_id = version_id.to_string();
    let fut = async move {
        return BoxResult::Ok(PerVersionMetadata {
            fabric_metadata: FabricVersionsMeta::fetch(&version_id).await?,
        });
    };

    let ctx = ctx.clone();
    return BackgroundTask::with_callback(
        fut,
        runtime,
        Box::new(move || {
            ctx.request_repaint();
        }),
    );
}

enum NewInstanceMetadataState<MetadataType> {
    NotFetched,
    Fetched(MetadataType),
    OfflineError,
    Error(String),
}

impl<MetadataType> NewInstanceMetadataState<MetadataType>
where
    MetadataType: Send,
{
    fn take_from_task(&mut self, task: BackgroundTask<BoxResult<MetadataType>>) {
        match task.take_result() {
            BackgroundTaskResult::Finished(result) => {
                *self = match result {
                    Ok(metadata) => NewInstanceMetadataState::Fetched(metadata),
                    Err(e) => {
                        if utils::is_connect_error(&e) {
                            NewInstanceMetadataState::OfflineError
                        } else {
                            NewInstanceMetadataState::Error(e.to_string())
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
            NewInstanceMetadataState::Error(e) => {
                ui.label(LangMessage::MetadataFetchError(e.clone()).to_string(lang));
            }
            NewInstanceMetadataState::Fetched(_) => {}
        }

        if matches!(
            self,
            NewInstanceMetadataState::Error(_) | NewInstanceMetadataState::OfflineError
        ) {
            if ui.button(LangMessage::Retry.to_string(lang)).clicked() {
                return true;
            }
        }

        false
    }
}

const VANILLA_LOADER: &str = "Vanilla";
const FABRIC_LOADER: &str = "Fabric";
const FORGE_LOADER: &str = "Forge";
const NEOFORGE_LOADER: &str = "NeoForge";

fn create_new_instance(
    runtime: &Runtime,
    ctx: &egui::Context,
    launcher_dir: &Path,
    version_manifest: &VersionManifest,
    instance_name: &str,
    minecraft_version: &str,
    loader: &str,
    loader_version: &str,
    assets_dir: &Path,
    progress_bar: Arc<dyn ProgressBar<LangMessage>>,
) -> BackgroundTask<BoxResult<VersionInfo>> {
    let launcher_dir = launcher_dir.to_path_buf();
    let version_manifest = version_manifest.clone();
    let instance_name = instance_name.to_string();
    let minecraft_version = minecraft_version.to_string();
    let loader = loader.to_string();
    let loader_version = loader_version.to_string();
    let assets_dir = assets_dir.to_path_buf();
    let fut = async move {
        let vanilla_info = get_vanilla_version_info(&version_manifest, &minecraft_version)?;

        let generator: Box<dyn VersionGenerator + Send>;
        match loader.as_str() {
            VANILLA_LOADER => {
                if !loader_version.is_empty() {
                    log::warn!("Ignoring loader version for vanilla version");
                }

                generator = Box::new(VanillaGenerator::new(
                    instance_name.to_string(),
                    vanilla_info,
                ));
            }

            FABRIC_LOADER => {
                generator = Box::new(FabricGenerator::new(
                    instance_name.to_string(),
                    vanilla_info,
                    Some(loader_version),
                ));
            }

            FORGE_LOADER => {
                generator = Box::new(ForgeGenerator::new(
                    instance_name.to_string(),
                    vanilla_info,
                    Loader::Forge,
                    Some(loader_version),
                    Arc::new(NoProgressBar),
                ));
            }

            NEOFORGE_LOADER => {
                generator = Box::new(ForgeGenerator::new(
                    instance_name.to_string(),
                    vanilla_info,
                    Loader::Neoforge,
                    Some(loader_version),
                    Arc::new(NoProgressBar),
                ));
            }

            _ => {
                return BoxResult::Err("Unknown loader".into());
            }
        }

        let generator_result = generator.generate(&launcher_dir).await?;

        const DOWNLOAD_SERVER_BASE: &str = "https://this-should-not-be-used.com";

        let extra_generator = ExtraMetadataGenerator::new(
            instance_name.to_string(),
            vec![],
            vec![],
            None,
            None,
            DOWNLOAD_SERVER_BASE.to_string(),
            generator_result.extra_libs_paths,
            AuthData::None,
        );
        let extra_generator_result = extra_generator.generate(&launcher_dir).await?;

        let version_info = get_version_info(
            &launcher_dir,
            &generator_result.metadata,
            &instance_name,
            DOWNLOAD_SERVER_BASE,
        )
        .await?;

        let complete_metadata = CompleteVersionMetadata::from_parts(
            instance_name.clone(),
            generator_result.metadata,
            Some(extra_generator_result.extra_metadata),
        );

        sync_instance(
            &complete_metadata,
            false,
            &launcher_dir,
            &assets_dir,
            progress_bar,
        )
        .await?;

        return BoxResult::Ok(version_info);
    };

    let ctx = ctx.clone();
    return BackgroundTask::with_callback(
        fut,
        runtime,
        Box::new(move || {
            ctx.request_repaint();
        }),
    );
}

pub struct NewInstanceState {
    window_open: bool,
    new_instance_name: String,
    instance_version: String,
    instance_loader: String,
    instance_loader_version: String,

    instance_metadata_task: Option<BackgroundTask<BoxResult<AllVersionsMetadata>>>,
    all_metadata_state: NewInstanceMetadataState<AllVersionsMetadata>,
    current_version_metadata_task: Option<BackgroundTask<BoxResult<PerVersionMetadata>>>,
    curent_metadata_state: NewInstanceMetadataState<PerVersionMetadata>,

    instance_generate_task: Option<BackgroundTask<BoxResult<VersionInfo>>>,
    instance_generate_error: Option<BoxError>,
    instance_sync_progress_bar: Arc<GuiProgressBar>,
}

impl NewInstanceState {
    pub fn new(runtime: &Runtime, ctx: &egui::Context) -> Self {
        let instance_sync_progress_bar = Arc::new(GuiProgressBar::new(ctx));

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
            instance_generate_error: None,
            instance_sync_progress_bar,
        }
    }

    pub fn take_new_instance(&mut self) -> Option<VersionInfo> {
        if let Some(task) = self.instance_generate_task.as_ref() {
            if task.has_result() {
                let task = self.instance_generate_task.take().unwrap();
                match task.take_result() {
                    BackgroundTaskResult::Finished(result) => match result {
                        Ok(version_info) => {
                            self.window_open = false;
                            self.instance_generate_error = None;
                            return Some(version_info);
                        }
                        Err(e) => {
                            self.instance_generate_error = Some(e);
                        }
                    },
                    BackgroundTaskResult::Cancelled => {
                        self.instance_generate_error = None;
                    }
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
        existing_names: &Vec<String>,
    ) {
        let lang = config.lang;

        if let Some(task) = self.instance_metadata_task.as_ref() {
            if task.has_result() {
                let task = self.instance_metadata_task.take();
                self.all_metadata_state.take_from_task(task.unwrap());
            }
        }
        if let Some(task) = self.current_version_metadata_task.as_mut() {
            if task.has_result() {
                let task = self.current_version_metadata_task.take();
                self.curent_metadata_state.take_from_task(task.unwrap());
                self.instance_loader = VANILLA_LOADER.to_string();
            }
        }

        if let Some(selected_instance_name) = &config.selected_instance_name {
            if ui.button("📂").clicked() {
                let launcher_dir = config.get_launcher_dir();
                let _ = open::that(get_instance_dir(&launcher_dir, selected_instance_name));
            }
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
                    egui::ComboBox::from_id_source("versions")
                        .selected_text(selected_version.clone())
                        .show_ui(ui, |ui| {
                            for version in versions.iter() {
                                ui.selectable_value(&mut selected_version, version.to_string(), version.to_string());
                            }
                        });

                    if selected_version != self.instance_version {
                        self.instance_version = selected_version;
                        self.current_version_metadata_task = Some(fetch_per_version_metadata(
                            &runtime,
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
                        &runtime,
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
                        if let Some(versions) = versions {
                            if !versions.is_empty() {
                                loaders.push(loader.to_string());
                            }
                        }
                    }

                    let mut new_instance_loader = self.instance_loader.clone();
                    ui.label(LangMessage::Loader.to_string(lang));
                    egui::ComboBox::from_id_source("loaders")
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
                                    version_name.insert(promotion.to_string(), format!("{} ({})", promotion, latest_type));
                                }
                            }
                        }

                        if self.instance_loader_version.is_empty() {
                            if self.instance_loader == FORGE_LOADER {
                                for latest_type in latest_types.iter() {
                                    if let Some(promotion) = all_metadata.forge_promotions.get_latest_version(&self.instance_version, latest_type) {
                                        self.instance_loader_version = promotion.to_string();
                                        break;
                                    }
                                }
                            }
                        }
                        if self.instance_loader_version.is_empty() {
                            if let Some(version) = versions.get(0) {
                                self.instance_loader_version = version.to_string();
                            }
                        }

                        ui.label(LangMessage::LoaderVersion.to_string(lang));
                        egui::ComboBox::from_id_source("loader_versions")
                            .selected_text(self.instance_loader_version.clone())
                            .show_ui(ui, |ui| {
                                for version in versions.iter() {
                                    ui.selectable_value(&mut self.instance_loader_version, version.to_string(), version_name.get(version).unwrap_or(&version).to_string());
                                }
                            });
                    });
                }

                if self.new_instance_name != "" && (self.instance_loader == VANILLA_LOADER || versions.contains_key(&self.instance_loader)) {
                    if existing_names.contains(&self.new_instance_name) {
                        ui.label(LangMessage::InstanceNameExists.to_string(lang));
                    } else {
                        ui.horizontal(|ui| {
                            if self.instance_generate_task.is_none() {
                                if ui.button(LangMessage::CreateInstance.to_string(lang)).clicked() {
                                    let task = create_new_instance(
                                        &runtime,
                                        ui.ctx(),
                                        &config.get_launcher_dir(),
                                        &all_metadata.vanilla_manifest,
                                        &self.new_instance_name,
                                        &self.instance_version,
                                        &self.instance_loader,
                                        &self.instance_loader_version,
                                        &config.get_assets_dir(),
                                        self.instance_sync_progress_bar.clone(),
                                    );
                                    self.instance_generate_task = Some(task);
                                }
                                if let Some(error) = &self.instance_generate_error {
                                    ui.label(if error.downcast_ref::<reqwest::Error>()
                                        .map_or(false, |e| e.is_connect())
                                    {
                                        LangMessage::InstanceGenerateErrorOffline
                                            .to_string(config.lang)
                                    } else {
                                        LangMessage::InstanceGenerateError(error.to_string())
                                            .to_string(config.lang)
                                    });
                                }
                            } else {
                                ui.label(LangMessage::CreatingInstance.to_string(lang));
                                if ui.button(LangMessage::Cancel.to_string(lang)).clicked() {
                                    self.instance_generate_task = None;
                                }
                            }
                        });
                        if self.instance_generate_task.is_some() {
                            self.instance_sync_progress_bar.render(ui, lang);
                        }
                    }
                }
            });
        self.window_open = new_instance_window_open;
    }
}
