use std::path::{Path, PathBuf};

pub fn get_assets_dir(output_dir: &Path) -> PathBuf {
    let assets_dir = output_dir.join("assets");
    if !assets_dir.exists() {
        std::fs::create_dir_all(&assets_dir).unwrap();
    }
    assets_dir
}

pub fn get_replaced_metadata_dir(output_dir: &Path) -> PathBuf {
    let replaced_manifests_dir = output_dir.join("versions_replaced");
    if !replaced_manifests_dir.exists() {
        std::fs::create_dir_all(&replaced_manifests_dir).unwrap();
    }
    replaced_manifests_dir
}
