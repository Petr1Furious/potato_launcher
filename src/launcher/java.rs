use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use futures::StreamExt;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use flate2::read::GzDecoder;
use tar::Archive;

use serde_json::Value;
#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

use crate::utils::get_temp_dir;
use crate::progress::ProgressBar;
use crate::lang::{Lang, get_loc};

#[derive(Debug, Deserialize)]
pub struct JavaInstallation {
    pub version: String,
    pub path: PathBuf,
}

lazy_static::lazy_static! {
    static ref JAVA_VERSION_RGX: Regex = Regex::new(r#""(.*)?""#).unwrap();
}

#[cfg(target_os = "windows")]
const JAVA_BINARY_NAME: &str = "java.exe";

#[cfg(not(target_os = "windows"))]
const JAVA_BINARY_NAME: &str = "java";

fn check_java(path: &Path) -> Option<JavaInstallation> {
    let path = if path.is_file() {
        path.to_path_buf()
    } else {
        which::which(path).ok()?
    };

    let output = Command::new(&path)
        .arg("-version")
        .output()
        .ok()?;

    let version_result = String::from_utf8_lossy(&output.stderr);
    let captures = JAVA_VERSION_RGX.captures(&version_result)?;

    let version = captures.get(1)?.as_str().to_string();
    Some(JavaInstallation {
        version,
        path,
    })
}

fn does_match(java: &JavaInstallation, required_version: &str) -> bool {
    java.version.starts_with(&format!("{}.", required_version))
}

#[cfg(target_os = "windows")]
fn find_java_in_registry(key_name: &str, subkey_suffix: &str, java_dir_key: &str) -> Vec<JavaInstallation> {
    let hk_local_machine = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hk_local_machine.open_subkey_with_flags(key_name, KEY_READ | KEY_ENUMERATE_SUB_KEYS).ok()?;

    let subkeys: Vec<String> = key.enum_keys().filter_map(Result::ok).collect();
    let mut res = Vec::new();

    for subkey in subkeys {
        let key_path = format!("{}\\{}{}", key_name, subkey, subkey_suffix);
        if let Ok(subkey) = hk_local_machine.open_subkey(&key_path) {
            if let Ok(java_dir_value) = subkey.get_value::<String, _>(java_dir_key) {
                let exe_path = Path::new(&java_dir_value).join("bin").join("java.exe");
                res.push(JavaInstallation {
                    version: subkey.to_string(),
                    path: exe_path,
                });
            }
        }
    }

    res
}

#[cfg(target_os = "windows")]
fn find_java_installations() -> Vec<JavaInstallation> {
    let mut res = Vec::new();

    let registry_paths = vec![
        (r"SOFTWARE\Eclipse Adoptium\JDK", r"\hotspot\MSI", "Path"),
        (r"SOFTWARE\Eclipse Adoptium\JRE", r"\hotspot\MSI", "Path"),
        (r"SOFTWARE\AdoptOpenJDK\JDK", r"\hotspot\MSI", "Path"),
        (r"SOFTWARE\AdoptOpenJDK\JRE", r"\hotspot\MSI", "Path"),
        (r"SOFTWARE\Eclipse Foundation\JDK", r"\hotspot\MSI", "Path"),
        (r"SOFTWARE\Eclipse Foundation\JRE", r"\hotspot\MSI", "Path"),
        (r"SOFTWARE\JavaSoft\JDK", "", "JavaHome"),
        (r"SOFTWARE\JavaSoft\JRE", "", "JavaHome"),
        (r"SOFTWARE\Microsoft\JDK", r"\hotspot\MSI", "Path"),
        (r"SOFTWARE\Azul Systems\Zulu", "", "InstallationPath"),
        (r"SOFTWARE\BellSoft\Liberica", "", "InstallationPath"),
    ];

    for (key, subkey_suffix, java_dir_key) in registry_paths {
        res.extend(find_java_in_registry(key, subkey_suffix, java_dir_key));
    }

    res
}

fn find_java_in_dir(dir: &Path, suffix: &str, startswith: &str) -> Vec<JavaInstallation> {
    let mut res = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let subdir = entry.path();
            if subdir.is_file() {
                continue;
            }
            if !startswith.is_empty() && !subdir.file_name().unwrap_or_default().to_string_lossy().starts_with(startswith) {
                continue;
            }
            if let Some(java) = check_java(&subdir.join(suffix).join("bin").join("java")) {
                res.push(java);
            }
        }
    }

    res
}

#[cfg(target_os = "linux")]
fn find_java_installations() -> Vec<JavaInstallation> {
    let mut res = Vec::new();
    res.extend(find_java_in_dir(Path::new("/usr/java"), "", ""));
    res.extend(find_java_in_dir(Path::new("/usr/lib/jvm"), "", ""));
    res.extend(find_java_in_dir(Path::new("/usr/lib64/jvm"), "", ""));
    res.extend(find_java_in_dir(Path::new("/usr/lib32/jvm"), "", ""));
    res.extend(find_java_in_dir(Path::new("/opt/jdk"), "", ""));
    res
}

#[cfg(target_os = "macos")]
fn find_java_installations() -> Vec<JavaInstallation> {
    let mut res = Vec::new();
    res.extend(find_java_in_dir(Path::new("/Library/Java/JavaVirtualMachines"), "Contents/Home", ""));
    res.extend(find_java_in_dir(Path::new("/System/Library/Java/JavaVirtualMachines"), "Contents/Home", ""));
    res.extend(find_java_in_dir(Path::new("/usr/local/opt"), "", "openjdk"));
    res.extend(find_java_in_dir(Path::new("/opt/homebrew/opt"), "", "openjdk"));
    res
}

#[derive(Debug)]
enum JavaDownloadError {
    UnsupportedArchitecture,
    UnsupportedOS,
    NoJavaVersionsAvailable,
}

impl std::error::Error for JavaDownloadError {}

impl std::fmt::Display for JavaDownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JavaDownloadError::UnsupportedArchitecture => write!(f, "Unsupported architecture"),
            JavaDownloadError::UnsupportedOS => write!(f, "Unsupported operating system"),
            JavaDownloadError::NoJavaVersionsAvailable => write!(f, "No Java versions available"),
        }
    }
}

fn get_java_download_params(required_version: &str) -> Result<String, JavaDownloadError> {
    let arch = match std::env::consts::ARCH {
        "x86_64" | "amd64" => "x64",
        "aarch64" => "aarch64",
        _ => return Err(JavaDownloadError::UnsupportedArchitecture),
    };

    let os_ = match std::env::consts::OS {
        "windows" => "windows",
        "linux" => "linux-glibc",
        "macos" => "macos",
        _ => return Err(JavaDownloadError::UnsupportedOS),
    };

    let params = format!(
        "java_version={}&os={}&arch={}&archive_type=tar.gz&java_package_type=jre&javafx_bundled=false&latest=true&release_status=ga",
        required_version, os_, arch
    );

    Ok(params)
}

pub async fn download_java(required_version: &str, java_dir: &Path, progress_bar: Arc<dyn ProgressBar + Send + Sync>, lang: &Lang) -> Result<JavaInstallation, Box<dyn std::error::Error>> {
    let client = Client::new();
    let query_str = get_java_download_params(required_version)?;
    let versions_url = format!("https://api.azul.com/metadata/v1/zulu/packages/?{}", query_str);

    let response = client.get(&versions_url).send().await?;
    let body = response.text().await?;
    let versions: Value = serde_json::from_str(&body)?;

    if versions.as_array().unwrap().is_empty() {
        return Err(Box::new(JavaDownloadError::NoJavaVersionsAvailable));
    }

    let version_url = versions[0]["download_url"].as_str().unwrap();
    let response = client.get(version_url).send().await?;

    let java_download_path = get_temp_dir().join("java_download.tar.gz");
    let mut file = fs::File::create(&java_download_path)?;

    let total_size = response.content_length().unwrap_or(0);
    progress_bar.set_length(total_size);
    progress_bar.set_message(get_loc(lang).downloading_java);

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        progress_bar.inc(chunk.len() as u64);
    }
    progress_bar.finish();

    let tar_gz = fs::File::open(&java_download_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    let target_dir = java_dir.join(required_version);
    fs::create_dir_all(&target_dir)?;
    
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        let path = path.strip_prefix(path.components().next().unwrap())?; // Remove the tar directory name
        let full_path = target_dir.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        entry.unpack(full_path)?;
    }

    let java_path = target_dir.join("bin").join(JAVA_BINARY_NAME);
    match check_java(&java_path) {
        Some(installation) => Ok(installation),
        None => Err(JavaDownloadError::NoJavaVersionsAvailable.into()),
    }
}

pub fn get_java(required_version: &str, java_dir: &Path) -> Option<JavaInstallation> {
    let mut installations = find_java_installations();
    
    if let Some(default_installation) = check_java(Path::new(JAVA_BINARY_NAME)) {
        installations.push(default_installation);
    }

    let java_dir = java_dir.join(required_version);
    if let Some(installation) = check_java(&java_dir.join("bin").join(JAVA_BINARY_NAME)) {
        installations.push(installation);
    }

    let matching = installations.into_iter().filter(|x| does_match(x, required_version)).next();
    if matching.is_some() {
        return matching;
    }

    None
}
