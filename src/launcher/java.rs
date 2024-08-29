use flate2::read::GzDecoder;
use futures::StreamExt;
use regex::Regex;
use reqwest::{Client, Url};
use serde::Deserialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tar::Archive;

use serde_json::Value;
#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

use crate::lang::LangMessage;
use crate::progress::ProgressBar;
use crate::utils::get_temp_dir;

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

fn get_installation(path: &Path) -> Option<JavaInstallation> {
    let path = if path.is_file() {
        path.to_path_buf()
    } else {
        which::which(path).ok()?
    };

    let mut cmd = Command::new(&path);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        use winapi::um::winbase::CREATE_NO_WINDOW;

        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd.arg("-version").output().ok()?;

    let version_result = String::from_utf8_lossy(&output.stderr);
    let captures = JAVA_VERSION_RGX.captures(&version_result)?;

    let version = captures.get(1)?.as_str().to_string();
    Some(JavaInstallation { version, path })
}

fn does_match(java: &JavaInstallation, required_version: &str) -> bool {
    java.version.starts_with(&format!("{}.", required_version))
}

pub fn check_java(required_version: &str, path: &Path) -> bool {
    if let Some(installation) = get_installation(path) {
        does_match(&installation, required_version)
    } else {
        false
    }
}

#[cfg(target_os = "windows")]
fn find_java_in_registry(
    key_name: &str,
    subkey_suffix: &str,
    java_dir_key: &str,
) -> Vec<JavaInstallation> {
    let hk_local_machine = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = match hk_local_machine
        .open_subkey_with_flags(key_name, KEY_READ | KEY_ENUMERATE_SUB_KEYS)
    {
        Ok(key) => key,
        Err(_) => return Vec::new(),
    };

    let subkeys: Vec<String> = key.enum_keys().filter_map(Result::ok).collect();
    let mut res = Vec::new();

    for subkey in subkeys {
        let key_path = format!("{}\\{}{}", key_name, subkey, subkey_suffix);
        if let Ok(subkey) = hk_local_machine.open_subkey(&key_path) {
            if let Ok(java_dir_value) = subkey.get_value::<String, _>(java_dir_key) {
                let exe_path = Path::new(&java_dir_value).join("bin").join("java.exe");
                if let Ok(version) = subkey.get_value::<String, _>("Version") {
                    res.push(JavaInstallation {
                        version,
                        path: exe_path,
                    });
                }
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

#[cfg(not(target_os = "windows"))]
fn find_java_in_dir(dir: &Path, suffix: &str, startswith: &str) -> Vec<JavaInstallation> {
    let mut res = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let subdir = entry.path();
            if subdir.is_file() {
                continue;
            }
            if !startswith.is_empty()
                && !subdir
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .starts_with(startswith)
            {
                continue;
            }
            if let Some(java) = get_installation(&subdir.join(suffix).join("bin").join("java")) {
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
    res.extend(find_java_in_dir(
        Path::new("/Library/Java/JavaVirtualMachines"),
        "Contents/Home",
        "",
    ));
    res.extend(find_java_in_dir(
        Path::new("/System/Library/Java/JavaVirtualMachines"),
        "Contents/Home",
        "",
    ));
    res.extend(find_java_in_dir(Path::new("/usr/local/opt"), "", "openjdk"));
    res.extend(find_java_in_dir(
        Path::new("/opt/homebrew/opt"),
        "",
        "openjdk",
    ));
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

fn get_java_download_params(
    required_version: &str,
    archive_type: &str,
) -> Result<String, JavaDownloadError> {
    let arch = match std::env::consts::ARCH {
        "x86_64" | "amd64" => "x64",
        "aarch64" => "aarch64",
        _ => return Err(JavaDownloadError::UnsupportedArchitecture),
    };

    let os = match std::env::consts::OS {
        "windows" => "windows",
        "linux" => "linux-glibc",
        "macos" => "macos",
        _ => return Err(JavaDownloadError::UnsupportedOS),
    };

    let params = format!(
        "java_version={}&os={}&arch={}&archive_type={}&java_package_type=jre&javafx_bundled=false&latest=true&release_status=ga",
        required_version, os, arch, archive_type
    );

    Ok(params)
}

pub async fn download_java(
    required_version: &str,
    java_dir: &Path,
    progress_bar: Arc<dyn ProgressBar + Send + Sync>,
) -> Result<JavaInstallation, Box<dyn std::error::Error>> {
    let client = Client::new();

    for archive_type in ["tar.gz", "zip"] {
        let query_str = get_java_download_params(required_version, archive_type)?;

        let versions_url = format!(
            "https://api.azul.com/metadata/v1/zulu/packages/?{}",
            query_str
        );

        let response = client.get(&versions_url).send().await?;
        let body = response.text().await?;
        let versions: Value = serde_json::from_str(&body)?;

        if versions.as_array().ok_or_else(|| "No versions array")?.is_empty() {
            continue;
        }

        let version_url = versions[0]["download_url"].as_str().ok_or_else(|| "No download URL")?;
        let response = client.get(version_url).send().await?;

        let java_download_path = get_temp_dir().join(format!("java_download.{}", archive_type));
        let mut file = fs::File::create(&java_download_path)?;

        let total_size = response.content_length().unwrap_or(0);
        progress_bar.set_length(total_size);
        progress_bar.set_message(LangMessage::DownloadingJava);

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk)?;
            progress_bar.inc(chunk.len() as u64);
        }
        progress_bar.finish();

        let target_dir = java_dir.join(required_version);
        if target_dir.exists() {
            fs::remove_dir_all(&target_dir)?;
        }

        let archive = fs::File::open(&java_download_path)?;
        if archive_type == "tar.gz" {
            let tar = GzDecoder::new(archive);
            let mut archive = Archive::new(tar);
            archive.unpack(&java_dir)?;
        } else {
            let mut archive = zip::ZipArchive::new(archive)?;
            archive.extract(&java_dir)?;
        }

        let url = Url::parse(version_url)?;
        let filename = url
            .path_segments()
            .and_then(|segments| segments.last())
            .ok_or_else(|| "No file name in URL")?
            .strip_suffix(&format!(".{}", archive_type))
            .ok_or_else(|| "No file extension in URL")?;
        fs::rename(java_dir.join(filename), &target_dir)?;

        let java_path = target_dir.join("bin").join(JAVA_BINARY_NAME);
        match get_installation(&java_path) {
            Some(installation) => return Ok(installation),
            None => {}
        }
    }

    Err(Box::new(JavaDownloadError::NoJavaVersionsAvailable))
}

pub fn get_java(required_version: &str, java_dir: &Path) -> Option<JavaInstallation> {
    let mut installations = find_java_installations();

    if let Some(default_installation) = get_installation(Path::new(JAVA_BINARY_NAME)) {
        installations.push(default_installation);
    }

    let java_dir = java_dir.join(required_version);
    if let Some(installation) = get_installation(&java_dir.join("bin").join(JAVA_BINARY_NAME)) {
        installations.push(installation);
    }

    let matching = installations
        .into_iter()
        .filter(|x| does_match(x, required_version))
        .next();
    if matching.is_some() {
        return matching;
    }

    None
}
