use std::path::{Path, PathBuf};

pub fn get_modpacks_dir(data_dir: &Path) -> PathBuf {
    let modpacks_dir = data_dir.join("modpacks");
    if !modpacks_dir.exists() {
        std::fs::create_dir_all(&modpacks_dir).expect("Failed to create modpacks directory");
    }
    modpacks_dir
}

pub fn get_minecraft_dir(data_dir: &Path, version_name: &str) -> PathBuf {
    let version_dir = get_modpacks_dir(data_dir).join(version_name);
    if !version_dir.exists() {
        std::fs::create_dir_all(&version_dir).expect("Failed to create minecraft directory");
    }
    version_dir
}

pub fn get_manifest_path(data_dir: &Path) -> PathBuf {
    get_modpacks_dir(data_dir).join("version_manifest.json")
}

pub fn get_java_dir(data_dir: &Path) -> PathBuf {
    let java_dir = data_dir.join("java");
    if !java_dir.exists() {
        std::fs::create_dir_all(&java_dir).expect("Failed to create java directory");
    }
    java_dir
}

pub fn get_logs_dir(data_dir: &Path) -> PathBuf {
    let logs_dir = data_dir.join("logs");
    if !logs_dir.exists() {
        std::fs::create_dir_all(&logs_dir).expect("Failed to create logs directory");
    }
    logs_dir
}

pub fn get_libraries_dir(data_dir: &Path, version_name: &str) -> PathBuf {
    let libraries_dir = get_minecraft_dir(data_dir, version_name).join("libraries");
    if !libraries_dir.exists() {
        std::fs::create_dir_all(&libraries_dir).expect("Failed to create libraries directory");
    }
    libraries_dir
}

pub fn get_natives_dir(data_dir: &Path, version_name: &str) -> PathBuf {
    let natives_dir = get_minecraft_dir(data_dir, version_name).join("natives");
    if !natives_dir.exists() {
        std::fs::create_dir_all(&natives_dir).expect("Failed to create natives directory");
    }
    natives_dir
}

pub fn get_versions_dir(data_dir: &Path) -> PathBuf {
    let versions_dir = get_modpacks_dir(data_dir).join("versions");
    if !versions_dir.exists() {
        std::fs::create_dir_all(&versions_dir).expect("Failed to create versions directory");
    }
    versions_dir
}

pub fn get_client_jar_path(data_dir: &Path, id: &str) -> PathBuf {
    let version_dir = get_versions_dir(data_dir).join(id);
    if !version_dir.exists() {
        std::fs::create_dir_all(&version_dir).expect("Failed to create version directory");
    }
    version_dir.join(format!("{}.jar", id))
}

pub fn get_versions_extra_dir(data_dir: &Path) -> PathBuf {
    let versions_extra_dir = get_modpacks_dir(data_dir).join("versions_extra");
    if !versions_extra_dir.exists() {
        std::fs::create_dir_all(&versions_extra_dir)
            .expect("Failed to create versions_extra directory");
    }
    versions_extra_dir
}

pub fn get_asset_index_path(assets_dir: &Path, asset_index: &str) -> PathBuf {
    let asset_index_dir = assets_dir.join("indexes");
    if !asset_index_dir.exists() {
        std::fs::create_dir_all(&asset_index_dir).expect("Failed to create asset index directory");
    }
    asset_index_dir.join(format!("{}.json", asset_index))
}
