use log::debug;
use maplit::hashmap;
use shared::paths::{
    get_client_jar_path, get_libraries_dir, get_logs_dir, get_minecraft_dir, get_natives_dir,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tokio::process::{Child, Command as TokioCommand};

use crate::auth::elyby::ELY_BY_BASE;
use crate::config::build_config;
use crate::config::runtime_config::{get_assets_dir, get_launcher_dir, Config};
use crate::version::complete_version_metadata::CompleteVersionMetadata;
use crate::version::overrides::with_overrides;
use crate::version::rules;

use shared::version::version_metadata;

use super::compat;

const AUTHLIB_INJECTOR_FILENAME: &str = "authlib-injector.jar";
const GC_OPTIONS: &[&str] = &[
    "-XX:+UnlockExperimentalVMOptions",
    "-XX:+UseG1GC",
    "-XX:G1NewSizePercent=20",
    "-XX:G1ReservePercent=20",
    "-XX:MaxGCPauseMillis=50",
    "-XX:G1HeapRegionSize=32M",
    "-XX:+DisableExplicitGC",
    "-XX:+AlwaysPreTouch",
    "-XX:+ParallelRefProcEnabled",
];

#[cfg(target_os = "windows")]
const PATHSEP: &str = ";";
#[cfg(not(target_os = "windows"))]
const PATHSEP: &str = ":";

fn replace_launch_config_variables(
    argument: String,
    variables: &HashMap<String, String>,
) -> String {
    variables.iter().fold(argument, |acc, (k, v)| {
        acc.replace(&format!("${{{}}}", k), v)
    })
}

fn process_args(
    args: &Vec<version_metadata::VariableArgument>,
    variables: &HashMap<String, String>,
) -> Vec<String> {
    let mut options = vec![];
    for arg in args {
        options.extend(
            rules::get_matching_values(arg)
                .iter()
                .map(|v| replace_launch_config_variables(v.to_string(), variables)),
        );
    }
    options
}

#[derive(thiserror::Error, Debug)]
pub enum LaunchError {
    #[error("Missing library {0}")]
    MissingLibrary(PathBuf),
    #[error("Invalid path {0}")]
    InvalidPath(PathBuf),
    #[error("Java path for version {0} not found")]
    JavaPathNotFound(String),
}

pub async fn launch(
    version_metadata: &CompleteVersionMetadata,
    config: &Config,
    online: bool,
) -> Result<Child, Box<dyn std::error::Error + Send + Sync>> {
    let base_version_metadata = &version_metadata.base;

    let launcher_dir = get_launcher_dir(config);
    let mut minecraft_dir = get_minecraft_dir(&launcher_dir, version_metadata.get_name());
    let libraries_dir = get_libraries_dir(&launcher_dir, version_metadata.get_name());
    let natives_dir = get_natives_dir(&launcher_dir, version_metadata.get_name());

    let minecraft_dir_short = minecraft_dir.clone();
    if cfg!(windows) {
        minecraft_dir = PathBuf::from(compat::win_get_long_path_name(
            minecraft_dir_short
                .to_str()
                .ok_or(Box::new(LaunchError::InvalidPath(
                    minecraft_dir_short.clone(),
                )))?,
        )?);
    }

    let mut used_library_paths = HashSet::new();
    let mut classpath = vec![];
    for library in with_overrides(
        base_version_metadata.get_libraries(),
        &base_version_metadata.hierarchy_ids,
    ) {
        let path = library.get_path(&libraries_dir);
        if let Some(path) = path {
            if !path.exists() {
                return Err(Box::new(LaunchError::MissingLibrary(path.clone())));
            }
            let path_str = path
                .to_str()
                .ok_or(Box::new(LaunchError::InvalidPath(path.clone())))?;

            if !used_library_paths.contains(path_str) {
                // vanilla mojang manifests have duplicates for some reason
                used_library_paths.insert(path_str.to_string());
                classpath.push(path_str.to_string());
            }
        }
    }

    classpath.push(
        get_client_jar_path(&launcher_dir, &version_metadata.base.id)
            .to_str()
            .unwrap()
            .to_string(),
    );

    let mut classpath_str = classpath.join(PATHSEP);
    if cfg!(windows) {
        classpath_str = classpath_str.replace("/", "\\");
    }

    let variables: HashMap<String, String> = hashmap! {
        "natives_directory".to_string() => natives_dir.to_str().unwrap().to_string(),
        "launcher_name".to_string() => "java-minecraft-launcher".to_string(),
        "launcher_version".to_string() => "1.6.84-j".to_string(),
        "classpath".to_string() => classpath_str,
        "classpath_separator".to_string() => PATHSEP.to_string(),
        "library_directory".to_string() => libraries_dir.to_str().unwrap().to_string(),
        "auth_player_name".to_string() => config.user_info.as_ref().unwrap().username.clone(),
        "version_name".to_string() => base_version_metadata.id.clone(),
        "game_directory".to_string() => minecraft_dir.to_str().unwrap().to_string(),
        "assets_root".to_string() => get_assets_dir(&config).to_str().unwrap().to_string(),
        "assets_index_name".to_string() => base_version_metadata.asset_index.id.clone(),
        "auth_uuid".to_string() => config.user_info.as_ref().unwrap().uuid.replace("-", ""),
        "auth_access_token".to_string() => config.token.as_ref().unwrap().clone(),
        "clientid".to_string() => "".to_string(),
        "auth_xuid".to_string() => "".to_string(),
        "user_type".to_string() => if online { "mojang" } else { "offline" }.to_string(),
        "version_type".to_string() => "release".to_string(),
        "resolution_width".to_string() => "925".to_string(),
        "resolution_height".to_string() => "530".to_string(),
        "user_properties".to_string() => "{}".to_string(),
    };

    let mut java_options = vec![
        GC_OPTIONS
            .iter()
            .map(|&s| s.to_string())
            .collect::<Vec<_>>(),
        vec![
            "-Xms512M".to_string(),
            format!("-Xmx{}", config.xmx),
            "-Duser.language=en".to_string(),
            "-Dfile.encoding=UTF-8".to_string(),
        ],
    ]
    .concat();

    if online {
        let authlib_injector_path = minecraft_dir.join(AUTHLIB_INJECTOR_FILENAME);
        if authlib_injector_path.exists() {
            let auth_server = if build_config::get_tgauth_base().is_some() {
                Some(build_config::get_tgauth_base().unwrap())
            } else if build_config::get_elyby_app_name().is_some() {
                Some(ELY_BY_BASE.to_string())
            } else {
                None
            };
            if let Some(auth_server) = auth_server {
                java_options.insert(
                    0,
                    format!(
                        "-javaagent:{}={}",
                        authlib_injector_path.to_str().unwrap(),
                        auth_server
                    ),
                );
            }
        }
    }

    java_options.extend(process_args(
        &base_version_metadata.arguments.jvm,
        &variables,
    ));
    let minecraft_options = process_args(&base_version_metadata.arguments.game, &variables);

    let java_path = config
        .java_paths
        .get(version_metadata.get_name())
        .ok_or_else(|| LaunchError::JavaPathNotFound(version_metadata.get_name().to_string()))?;

    debug!(
        "Launching java {} with arguments {:?}",
        java_path, java_options
    );
    debug!("Main class: {}", base_version_metadata.main_class);
    debug!("Game arguments: {:?}", minecraft_options);

    let mut cmd = TokioCommand::new(java_path);
    cmd.args(&java_options)
        .arg(&base_version_metadata.main_class)
        .args(&minecraft_options)
        .current_dir(minecraft_dir_short);

    // for some reason this is needed on macOS for minecraft process not to crash with
    // "Assertion failed: (count <= len && "snprintf() output has been truncated"), function LOAD_ERROR, file dispatch.c, line 74."
    std::env::remove_var("DYLD_FALLBACK_LIBRARY_PATH");

    let file =
        std::fs::File::create(get_logs_dir(&launcher_dir).join("latest_minecraft_launch.log"))?;
    cmd.stdout(file.try_clone()?);
    cmd.stderr(file);

    #[cfg(target_os = "windows")]
    {
        use winapi::um::winbase::CREATE_NO_WINDOW;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    Ok(cmd.spawn()?)
}
