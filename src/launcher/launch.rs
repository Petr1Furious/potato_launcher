use std::collections::HashMap;
use std::path::PathBuf;
use maplit::hashmap;
use tokio::fs;
use tokio::process::{Child, Command as TokioCommand};

use crate::auth::base::{get_auth_provider, AuthProvider};
use crate::auth::elyby::{ElyByAuthProvider, ELY_BY_BASE};
use crate::auth::telegram::TGAuthProvider;
use crate::config::build_config;
use crate::config::runtime_config::{get_assets_dir, get_minecraft_dir, Config};
use crate::modpack::index::ModpackIndex;

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

fn apply_arg(arg: &serde_json::Value) -> bool {
    let arg = arg.as_object().unwrap();

    if arg.get("value") == Some(&serde_json::json!(["-Dos.name=Windows 10", "-Dos.version=10.0"])) {
        return false;
    }

    if !arg.contains_key("rules") {
        return true;
    }

    let rules = arg.get("rules").unwrap().as_array().unwrap();
    if rules.len() != 1 {
        return false;
    }

    let rule = &rules[0];
    if rule.get("action") != Some(&serde_json::json!("allow")) {
        return false;
    }

    if let Some(os) = rule.get("os") {
        if let Some(os_name) = os.get("name") {
            match os_name.as_str().unwrap() {
                "windows" if cfg!(windows) => return true,
                "osx" if cfg!(target_os = "macos") => return true,
                "linux" if cfg!(target_os = "linux") => return true,
                _ => return false,
            }
        }
    } else if let Some(features) = rule.get("features") {
        if features.get("has_custom_resolution") == Some(&serde_json::json!(true)) {
            return true;
        }
    }

    false
}

fn replace_launch_config_variables(argument: String, variables: &HashMap<String, String>) -> String {
    variables.iter().fold(argument, |acc, (k, v)| acc.replace(&format!("${{{}}}", k), v))
}

fn library_name_to_path(full_name: &str) -> String {
    let mut parts: Vec<&str> = full_name.split(':').collect();
    if parts.len() != 4 {
        parts.push("");
    }
    let (pkg, name, version, suffix) = (parts[0], parts[1], parts[2], parts[3]);
    let pkg_path = pkg.replace('.', "/");
    let suffix = if suffix.is_empty() { "".to_string() } else { format!("-{}", suffix) };
    format!("libraries/{}/{}/{}/{}-{}{}.jar", pkg_path, name, version, name, version, suffix)
}

fn process_args(args: &[serde_json::Value], variables: &HashMap<String, String>) -> Vec<String> {
    let mut options = vec![];
    for arg in args {
        if apply_arg(arg) {
            if let Some(values) = arg.get("value").and_then(|v| v.as_array()) {
                options.extend(values.iter().map(|v| replace_launch_config_variables(v.as_str().unwrap().to_string(), variables)));
            } else if let Some(value) = arg.get("value").and_then(|v| v.as_str()) {
                options.push(replace_launch_config_variables(value.to_string(), variables));
            }
        }
    }
    options
}

pub async fn launch(modpack_index: ModpackIndex, config: &Config, online: bool) -> Result<Child, Box<dyn std::error::Error>> {
    let mut mc_dir = get_minecraft_dir(&config, &modpack_index.modpack_name);
    let mc_dir_short = mc_dir.clone();
    if cfg!(windows) {
        mc_dir = PathBuf::from(compat::win_get_long_path_name(mc_dir_short.to_str().unwrap())?);
    }
    fs::create_dir_all(mc_dir.join("natives")).await?;

    let mut classpath = vec![];
    for arg in &modpack_index.libraries {
        if arg.get("downloadOnly").is_some() {
            continue;
        }
        if apply_arg(arg) {
            classpath.push(mc_dir.join(library_name_to_path(arg.get("name").unwrap().as_str().unwrap())).to_str().unwrap().to_string());
        }
    }
    classpath.push(mc_dir.join(&modpack_index.client_filename).to_str().unwrap().to_string());

    let variables: HashMap<String, String> = hashmap! {
        "natives_directory".to_string() => mc_dir.join("natives").to_str().unwrap().to_string(),
        "launcher_name".to_string() => "java-minecraft-launcher".to_string(),
        "launcher_version".to_string() => "1.6.84-j".to_string(),
        "classpath".to_string() => classpath.join(PATHSEP),
        "classpath_separator".to_string() => PATHSEP.to_string(),
        "library_directory".to_string() => mc_dir.join("libraries").to_str().unwrap().to_string(),
        "auth_player_name".to_string() => config.user_info.as_ref().unwrap().username.clone(),
        "version_name".to_string() => modpack_index.minecraft_version.clone(),
        "game_directory".to_string() => mc_dir.to_str().unwrap().to_string(),
        "assets_root".to_string() => get_assets_dir(&config).to_str().unwrap().to_string(),
        "assets_index_name".to_string() => modpack_index.asset_index.clone(),
        "auth_uuid".to_string() => config.user_info.as_ref().unwrap().uuid.replace("-", ""),
        "auth_access_token".to_string() => config.token.as_ref().unwrap().clone(),
        "clientid".to_string() => "".to_string(),
        "auth_xuid".to_string() => "".to_string(),
        "user_type".to_string() => if online { "mojang" } else { "offline" }.to_string(),
        "version_type".to_string() => "release".to_string(),
        "resolution_width".to_string() => "925".to_string(),
        "resolution_height".to_string() => "530".to_string(),
    };

    let mut java_options = vec![
        GC_OPTIONS.iter().map(|&s| s.to_string()).collect::<Vec<_>>(),
        vec![
            "-Xms512M".to_string(),
            format!("-Xmx{}", config.xmx),
            "-Duser.language=en".to_string(),
            "-Dfile.encoding=UTF-8".to_string(),
        ],
    ].concat();

    if online {
        let auth_provider = get_auth_provider(config.lang.clone());
        if auth_provider.as_any().downcast_ref::<ElyByAuthProvider>().is_some() {
            java_options.insert(0, format!("-javaagent:{}={}", mc_dir.join(AUTHLIB_INJECTOR_FILENAME).to_str().unwrap(), ELY_BY_BASE));
        } else if auth_provider.as_any().downcast_ref::<TGAuthProvider>().is_some() {
            java_options.insert(0, format!("-javaagent:{}={}", mc_dir.join(AUTHLIB_INJECTOR_FILENAME).to_str().unwrap(), build_config::get_tgauth_base().unwrap()));
        }
    }

    java_options.extend(process_args(&modpack_index.java_args, &variables));
    let minecraft_options = process_args(&modpack_index.game_args, &variables);

    let mut cmd = TokioCommand::new(config.java_paths.get(&modpack_index.modpack_name).unwrap().clone());
    cmd.args(&java_options).arg(&modpack_index.main_class).args(&minecraft_options).current_dir(mc_dir_short);

    // for some reason this is needed on macOS for minecraft process not to crash with
    // "Assertion failed: (count <= len && "snprintf() output has been truncated"), function LOAD_ERROR, file dispatch.c, line 74."
    std::env::remove_var("DYLD_FALLBACK_LIBRARY_PATH");

    Ok(cmd.spawn()?)
}
