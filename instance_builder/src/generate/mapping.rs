use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use shared::utils::BoxResult;

pub fn get_mapping(
    output_dir: &Path,
    work_dir: &Path,
    workdir_paths: &Vec<PathBuf>,
) -> BoxResult<HashMap<PathBuf, PathBuf>> {
    let mut mapping = HashMap::new();

    for path in workdir_paths {
        let relative_path = path.strip_prefix(work_dir)?;
        let output_path = output_dir.join(relative_path);
        mapping.insert(output_path, path.to_path_buf());
    }

    Ok(mapping)
}
