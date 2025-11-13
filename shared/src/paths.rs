use std::{
    fs,
    path::{Path, PathBuf},
};

fn created(dir: PathBuf) -> PathBuf {
    fs::create_dir_all(&dir).expect("Failed to create directory");
    dir
}

fn parent_created(file: PathBuf) -> PathBuf {
    created(file.parent().unwrap().to_path_buf());
    file
}

pub fn get_rel_instances_dir() -> PathBuf {
    PathBuf::from("instances")
}

pub fn get_instances_dir(data_dir: &Path) -> PathBuf {
    let old_instances_dir = data_dir.join("modpacks");
    let instances_dir = data_dir.join(get_rel_instances_dir());
    if old_instances_dir.exists() && !instances_dir.exists() {
        fs::rename(old_instances_dir, &instances_dir).expect("Failed to rename modpacks directory");
    }
    created(instances_dir)
}

pub fn get_rel_instance_dir(version_name: &str) -> PathBuf {
    get_rel_instances_dir().join(version_name)
}

pub fn get_instance_dir(data_dir: &Path, version_name: &str) -> PathBuf {
    let old_instances_dir = data_dir.join("modpacks");
    let instances_dir = data_dir.join(get_rel_instances_dir());
    if old_instances_dir.exists() && !instances_dir.exists() {
        fs::rename(old_instances_dir, &instances_dir).expect("Failed to rename modpacks directory");
    }
    created(data_dir.join(get_rel_instance_dir(version_name)))
}

pub fn get_rel_minecraft_dir(version_name: &str) -> PathBuf {
    get_rel_instance_dir(version_name).join("minecraft")
}

pub fn get_minecraft_dir(data_dir: &Path, dir_name: &str) -> PathBuf {
    created(data_dir.join(get_rel_minecraft_dir(dir_name)))
}

pub fn get_local_instances_path(data_dir: &Path) -> PathBuf {
    parent_created(data_dir.join("local_instances.json"))
}

pub fn get_instance_meta_path(data_dir: &Path, dir_name: &str) -> PathBuf {
    parent_created(get_instance_dir(data_dir, dir_name).join("meta.json"))
}

pub fn get_auth_data_path(data_dir: &Path) -> PathBuf {
    parent_created(data_dir.join("auth_data.json"))
}

pub fn get_java_dir(data_dir: &Path) -> PathBuf {
    created(data_dir.join("java"))
}

pub fn get_logs_dir(data_dir: &Path) -> PathBuf {
    created(data_dir.join("logs"))
}

pub fn get_libraries_dir(data_dir: &Path) -> PathBuf {
    created(data_dir.join("libraries"))
}

pub fn get_natives_dir(data_dir: &Path, id: &str) -> PathBuf {
    created(data_dir.join("natives").join(id))
}

pub fn get_rel_versions_dir() -> PathBuf {
    PathBuf::from("versions")
}

pub fn get_versions_dir(data_dir: &Path) -> PathBuf {
    created(data_dir.join(get_rel_versions_dir()))
}

pub fn get_rel_metadata_path(version_id: &str) -> PathBuf {
    PathBuf::from(version_id).join(format!("{version_id}.json"))
}

pub fn get_metadata_path(versions_dir: &Path, version_id: &str) -> PathBuf {
    parent_created(versions_dir.join(get_rel_metadata_path(version_id)))
}

pub fn get_client_jar_path(data_dir: &Path, id: &str) -> PathBuf {
    parent_created(
        get_versions_dir(data_dir)
            .join(id)
            .join(format!("{id}.jar")),
    )
}

pub fn get_rel_versions_extra_dir() -> PathBuf {
    PathBuf::from("versions_extra")
}

pub fn get_versions_extra_dir(data_dir: &Path) -> PathBuf {
    created(data_dir.join(get_rel_versions_extra_dir()))
}

pub fn get_rel_extra_metadata_path(version_name: &str) -> PathBuf {
    PathBuf::from(format!("{version_name}.json"))
}

pub fn get_extra_metadata_path(versions_extra_dir: &Path, version_name: &str) -> PathBuf {
    parent_created(versions_extra_dir.join(get_rel_extra_metadata_path(version_name)))
}

pub fn get_asset_index_path(assets_dir: &Path, asset_index: &str) -> PathBuf {
    parent_created(
        assets_dir
            .join("indexes")
            .join(format!("{asset_index}.json")),
    )
}

pub fn get_assets_object_path(assets_dir: &Path) -> PathBuf {
    created(assets_dir.join("objects"))
}

const AUTHLIB_INJECTOR_NAME: &str = "authlib-injector.jar";

pub fn get_authlib_injector_path(data_dir: &Path) -> PathBuf {
    parent_created(get_libraries_dir(data_dir).join(AUTHLIB_INJECTOR_NAME))
}
