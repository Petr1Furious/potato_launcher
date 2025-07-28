use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    io::Write as _,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    files,
    java::{download_java, get_java},
    paths::{get_java_dir, get_libraries_dir, get_metadata_path, get_versions_dir},
    progress::ProgressBar,
    version::{version_manifest::VersionInfo, version_metadata::VersionMetadata},
};
use async_trait::async_trait;
use log::{debug, error, info, warn};
use reqwest::Client;
use serde::Deserialize;
use tokio::io::AsyncWriteExt as _;

use super::generator::{GeneratorResult, VersionGenerator};

const FORGE_MAVEN_METADATA_URL: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json";

const FORGE_PROMOTIONS_URL: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";

const NEOFORGE_MAVEN_METADATA_URL: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";

#[derive(Debug, Deserialize)]
pub struct ForgeMavenMetadata {
    versions: HashMap<String, Vec<String>>,
}

impl ForgeMavenMetadata {
    pub async fn fetch() -> anyhow::Result<Self> {
        let client = Client::new();
        let response = client
            .get(FORGE_MAVEN_METADATA_URL)
            .send()
            .await?
            .error_for_status()?;
        Ok(ForgeMavenMetadata {
            versions: response.json().await?,
        })
    }

    pub fn get_matching_versions(&self, minecraft_version: &str) -> Vec<String> {
        self.versions
            .get(minecraft_version)
            .cloned()
            .unwrap_or(vec![])
            .into_iter()
            .rev()
            .filter_map(|version| {
                version
                    .strip_prefix(&format!("{minecraft_version}-"))
                    .map(|forge_version| forge_version.to_string())
            })
            .collect()
    }

    fn has_version(&self, minecraft_version: &str, forge_version: &str) -> bool {
        self.versions
            .get(minecraft_version)
            .is_some_and(|versions| {
                versions.contains(&format!("{minecraft_version}-{forge_version}"))
            })
    }
}

#[derive(Debug, Deserialize)]
struct Versions {
    version: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Versioning {
    versions: Versions,
}

#[derive(Debug, Deserialize)]
pub struct NeoforgeMavenMetadata {
    versioning: Versioning,
}

impl NeoforgeMavenMetadata {
    pub async fn fetch() -> anyhow::Result<Self> {
        let client = Client::new();
        let response = client
            .get(NEOFORGE_MAVEN_METADATA_URL)
            .send()
            .await?
            .error_for_status()?;
        let metadata: NeoforgeMavenMetadata = serde_xml_rs::from_str(&response.text().await?)?;
        Ok(metadata)
    }

    pub fn get_matching_versions(&self, minecraft_version: &str) -> Vec<String> {
        let mut mc_version_parts: Vec<&str> = minecraft_version.split('.').collect();
        if mc_version_parts.len() < 2 {
            return vec![];
        }
        if mc_version_parts.len() == 2 {
            mc_version_parts.push("0");
        }

        let mc_version_prefix = format!("{}.{}", mc_version_parts[1], mc_version_parts[2]);
        self.versioning
            .versions
            .version
            .iter()
            .rev()
            .filter(|&version| version.starts_with(&mc_version_prefix))
            .cloned()
            .collect()
    }

    pub fn get_latest_matching_version(&self, minecraft_version: &str) -> Option<String> {
        self.get_matching_versions(minecraft_version)
            .into_iter()
            .max_by(|a, b| {
                let a_parts: Vec<u32> = a
                    .split(|c: char| !c.is_ascii_digit())
                    .filter_map(|s| s.parse().ok())
                    .collect();
                let b_parts: Vec<u32> = b
                    .split(|c: char| !c.is_ascii_digit())
                    .filter_map(|s| s.parse().ok())
                    .collect();
                a_parts.cmp(&b_parts)
            })
    }

    pub fn has_version(&self, version: &str) -> bool {
        self.versioning
            .versions
            .version
            .contains(&version.to_string())
    }
}

#[derive(Deserialize)]
pub struct ForgePromotions {
    promos: HashMap<String, String>,
}

impl ForgePromotions {
    pub async fn fetch() -> anyhow::Result<Self> {
        let client = Client::new();
        let response = client
            .get(FORGE_PROMOTIONS_URL)
            .send()
            .await?
            .error_for_status()?;
        let promotions: ForgePromotions = response.json().await?;
        Ok(promotions)
    }

    pub fn get_latest_version(
        &self,
        minecraft_version: &str,
        version_type: &str,
    ) -> Option<String> {
        self.promos
            .get(&format!("{minecraft_version}-{version_type}"))
            .cloned()
    }
}

const FORGE_INSTALLER_BASE_URL: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge/";

const NEOFORGE_INSTALLER_BASE_URL: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/";

async fn download_forge_installer(
    full_version: &str,
    work_dir: &Path,
    loader: &Loader,
) -> anyhow::Result<PathBuf> {
    let filename = format!("{loader:?}-{full_version}-installer.jar");
    let forge_installer_url = match loader {
        Loader::Forge => format!("{FORGE_INSTALLER_BASE_URL}{full_version}/{filename}"),
        Loader::Neoforge => format!("{NEOFORGE_INSTALLER_BASE_URL}{full_version}/{filename}"),
    };
    let forge_installer_path = work_dir.join(filename);

    let client = Client::new();
    let response = client
        .get(&forge_installer_url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let mut file = tokio::fs::File::create(&forge_installer_path).await?;
    file.write_all(&response).await?;

    Ok(forge_installer_path)
}

#[derive(Deserialize)]
struct ProfileInfo {
    #[serde(rename = "lastVersionId")]
    last_version_id: String,
}

#[derive(Deserialize)]
pub struct LauncherProfiles {
    profiles: HashMap<String, ProfileInfo>,
}

pub enum Loader {
    Forge,
    Neoforge,
}

impl Display for Loader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Loader::Forge => write!(f, "Forge"),
            Loader::Neoforge => write!(f, "Neoforge"),
        }
    }
}

impl Debug for Loader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Loader::Forge => write!(f, "forge"),
            Loader::Neoforge => write!(f, "neoforge"),
        }
    }
}

pub struct ForgeGenerator {
    version_name: String,
    vanilla_version_info: VersionInfo,
    loader: Loader,
    loader_version: Option<String>,
    progress_bar: Arc<dyn ProgressBar<&'static str>>,
}

impl ForgeGenerator {
    pub fn new(
        version_name: String,
        vanilla_version_info: VersionInfo,
        loader: Loader,
        loader_version: Option<String>,
        progress_bar: Arc<dyn ProgressBar<&'static str>>,
    ) -> Self {
        Self {
            version_name,
            vanilla_version_info,
            loader,
            loader_version,
            progress_bar,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ForgeError {
    #[error("Forge version {0} not found for minecraft {1}")]
    ForgeVersionNotFound(String, String),
    #[error("No forge profiles found")]
    NoForgeProfiles,
}

pub async fn get_forge_version(
    minecraft_version: &str,
    loader_version: &Option<String>,
    loader: &Loader,
) -> anyhow::Result<String> {
    match loader {
        Loader::Forge => {
            let forge_promotions = ForgePromotions::fetch().await?;

            let forge_version = match loader_version {
                Some(version) => version.to_string(),
                None => {
                    const FORGE_DEFAULT: &str = "recommended";
                    info!("Version not set, using \"{FORGE_DEFAULT}\"");
                    forge_promotions
                        .get_latest_version(minecraft_version, FORGE_DEFAULT)
                        .ok_or(ForgeError::ForgeVersionNotFound(
                            FORGE_DEFAULT.to_string(),
                            minecraft_version.to_string(),
                        ))?
                }
            };

            let forge_maven_metadata = ForgeMavenMetadata::fetch().await?;
            if forge_maven_metadata.has_version(minecraft_version, &forge_version) {
                return Ok(forge_version);
            }
            let version_with_suffix = format!("{forge_version}-{minecraft_version}");
            if forge_maven_metadata.has_version(minecraft_version, &version_with_suffix) {
                return Ok(version_with_suffix);
            }
        }
        Loader::Neoforge => {
            let neoforge_maven_metadata = NeoforgeMavenMetadata::fetch().await?;

            let neoforge_version = match loader_version {
                Some(version) => version.to_string(),
                None => {
                    info!("Version not set, using latest");
                    neoforge_maven_metadata
                        .get_latest_matching_version(minecraft_version)
                        .ok_or(ForgeError::ForgeVersionNotFound(
                            "neoforge:latest".to_string(),
                            minecraft_version.to_string(),
                        ))?
                }
            };

            if neoforge_maven_metadata.has_version(&neoforge_version) {
                return Ok(neoforge_version);
            }
        }
    };

    let forge_version = loader_version.as_deref().unwrap_or("default");
    error!("{loader} version {forge_version} not found for minecraft {minecraft_version}");
    Err(
        ForgeError::ForgeVersionNotFound(forge_version.to_string(), minecraft_version.to_string())
            .into(),
    )
}

pub async fn get_vanilla_java_version(
    vanilla_metadata: &VersionMetadata,
) -> anyhow::Result<Option<String>> {
    Ok(vanilla_metadata
        .java_version
        .as_ref()
        .map(|v| v.major_version.to_string()))
}

// trick forge installer into thinking that the folder is actually a minecraft instance folder
pub fn trick_forge(forge_work_dir: &Path, minecraft_version: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(forge_work_dir.join("versions").join(minecraft_version))?;
    let mut file = std::fs::File::create(forge_work_dir.join("launcher_profiles.json"))?;
    let _ = file.write(b"{\"profiles\":{}}")?;
    Ok(())
}

pub fn get_full_version(minecraft_version: &str, forge_version: &str) -> String {
    format!("{minecraft_version}-{forge_version}")
}

// workaround for windows weirdness
fn to_abs_path_str(path: &Path) -> anyhow::Result<String> {
    let canonical = path.canonicalize()?;
    let path_str = canonical.to_string_lossy();

    #[cfg(windows)]
    {
        const VERBATIM_PREFIX: &str = r"\\?\";
        if let Some(stripped) = path_str.strip_prefix(VERBATIM_PREFIX) {
            Ok(stripped.to_string())
        } else {
            Ok(path_str.to_string())
        }
    }

    #[cfg(not(windows))]
    {
        Ok(path_str.to_string())
    }
}

async fn run_forge_command(
    java_path: &Path,
    forge_installer_path: &Path,
    forge_work_dir: &Path,
) -> anyhow::Result<()> {
    let mut cmd = tokio::process::Command::new(&to_abs_path_str(java_path)?);
    cmd.current_dir(&to_abs_path_str(forge_work_dir)?)
        .arg("-jar")
        .arg(&to_abs_path_str(forge_installer_path)?)
        .arg("--installClient")
        .arg(".");
    info!("Running forge installer: {cmd:?}");

    let output = cmd.output().await?;
    if !output.status.success() {
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        if stderr_str.contains("'installClient' is not a recognized option") {
            info!("Retrying without '--installClient' argument.");
            let mut retry_cmd = tokio::process::Command::new(&to_abs_path_str(java_path)?);
            retry_cmd
                .current_dir(&to_abs_path_str(forge_work_dir)?)
                .arg("-jar")
                .arg(&to_abs_path_str(forge_installer_path)?);
            let retry_output = retry_cmd.output().await?;
            if !retry_output.status.success() {
                return Err(anyhow::anyhow!(
                    "Command failed: {:?}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        } else {
            error!("Command failed: {output:?}");
            return Err(anyhow::anyhow!(stderr_str.to_string()));
        }
    }

    Ok(())
}

pub async fn install_forge<M>(
    forge_work_dir: &Path,
    java_dir: &Path,
    forge_version: &str,
    vanilla_metadata: &VersionMetadata,
    loader: &Loader,
    progress_bar: Arc<dyn ProgressBar<M>>,
) -> anyhow::Result<String> {
    std::fs::create_dir_all(forge_work_dir)?;

    let minecraft_version = &vanilla_metadata.id;

    let lock_file = forge_work_dir.join("forge.lock");

    if !lock_file.exists() {
        let java_version = get_vanilla_java_version(vanilla_metadata)
            .await?
            .map_or_else(
                || {
                    warn!("Java version not found, using default");
                    "8".to_string()
                },
                |v| v,
            );

        info!("Getting java {}", &java_version);
        let java_installation;
        if let Some(existing_java_installation) = get_java(&java_version, java_dir).await {
            java_installation = existing_java_installation;
        } else {
            info!("Java installation not found, downloading");

            java_installation = download_java(&java_version, java_dir, progress_bar).await?;
        }

        info!("Downloading forge installer");
        let full_version = match loader {
            Loader::Forge => get_full_version(minecraft_version, forge_version),
            Loader::Neoforge => forge_version.to_string(),
        };
        let forge_installer_path =
            download_forge_installer(&full_version, forge_work_dir, loader).await?;

        trick_forge(forge_work_dir, minecraft_version)?;

        run_forge_command(
            &java_installation.path,
            &forge_installer_path,
            forge_work_dir,
        )
        .await?;
    } else {
        info!("Forge {forge_version} already present, skipping installation");
    }

    let launcher_profiles_path = forge_work_dir.join("launcher_profiles.json");
    let launcher_profiles_content = std::fs::read_to_string(&launcher_profiles_path)?;
    let launcher_profiles: LauncherProfiles = serde_json::from_str(&launcher_profiles_content)?;

    let id = launcher_profiles
        .profiles
        .values()
        .next()
        .ok_or(ForgeError::NoForgeProfiles)?
        .last_version_id
        .clone();

    if !lock_file.exists() {
        std::fs::File::create(lock_file)?;
    }

    Ok(id)
}

#[async_trait]
impl VersionGenerator for ForgeGenerator {
    async fn generate(&self, work_dir: &Path) -> anyhow::Result<GeneratorResult> {
        let minecraft_version = self.vanilla_version_info.id.clone();

        info!(
            "Generating {} instance \"{}\", minecraft version {}",
            self.loader, self.version_name, minecraft_version
        );

        info!("Downloading vanilla version metadata");
        let vanilla_metadata = VersionMetadata::read_or_download(
            &self.vanilla_version_info.get_parent_metadata_info(),
            &get_versions_dir(work_dir),
        )
        .await?;

        let forge_version =
            get_forge_version(&minecraft_version, &self.loader_version, &self.loader).await?;

        info!("Using {} version {}", self.loader, &forge_version);

        let installer_work_dir = work_dir
            .join(format!(".{:?}", self.loader))
            .join(get_full_version(&minecraft_version, &forge_version));
        let id = install_forge(
            &installer_work_dir,
            &get_java_dir(work_dir),
            &forge_version,
            &vanilla_metadata,
            &self.loader,
            self.progress_bar.clone(),
        )
        .await?;

        let versions_dir_from = installer_work_dir.join("versions");
        let versions_dir_to = get_versions_dir(work_dir);

        info!("Copying version metadata");
        let metadata_from = versions_dir_from.join(&id).join(format!("{id}.json"));
        let metadata_to = get_metadata_path(&versions_dir_to, &id);
        std::fs::copy(metadata_from, metadata_to)?;

        let forge_metadata = VersionMetadata::read_local(&versions_dir_to, &id).await?;

        let installer_libraries_dir = installer_work_dir.join("libraries");

        let extra_libs_paths = files::get_files_in_dir(&installer_libraries_dir)?
            .into_iter()
            .filter_map(|path| {
                let extension = path.extension().and_then(|ext| ext.to_str());
                if path.is_file() && extension == Some("jar") {
                    Some(
                        path.strip_prefix(&installer_libraries_dir)
                            .unwrap()
                            .to_path_buf(),
                    )
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        info!(
            "Found {} extra {} libs",
            extra_libs_paths.len(),
            self.loader
        );
        debug!("Extra {} libs: {:?}", self.loader, extra_libs_paths);

        // copy extra forge libs to work dir
        let forge_installer_libraries_dir = installer_work_dir.join("libraries");
        let libraries_dir = get_libraries_dir(work_dir);
        let extra_libs_paths = extra_libs_paths
            .into_iter()
            .map(|lib_path| {
                let lib_dest = libraries_dir.join(&lib_path);
                std::fs::create_dir_all(lib_dest.parent().unwrap())?;
                std::fs::copy(forge_installer_libraries_dir.join(&lib_path), &lib_dest)?;
                Ok(lib_dest)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        forge_metadata.save(&versions_dir_to).await?;

        info!(
            "{} version \"{}\" generated",
            self.loader, self.version_name
        );

        Ok(GeneratorResult {
            metadata: vec![vanilla_metadata, forge_metadata],
            extra_libs_paths,
        })
    }
}
