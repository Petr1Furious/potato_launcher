use log::{info, warn};
use serde::Deserialize;
use shared::version::version_metadata::{Library, LibraryDownloads, Rule};
use std::collections::{HashMap, HashSet};

use crate::config::build_config;

#[derive(Deserialize)]
pub struct Replacement {
    pub libraries: Vec<Library>,
    pub version: String,
}

#[derive(Deserialize)]
pub struct LibraryOverrides {
    lwjgl_group_ids: HashSet<String>,
    overrides: Vec<Replacement>,
}

lazy_static::lazy_static! {
    static ref LIBRARY_OVERRIDES: LibraryOverrides = {
        let overrides = build_config::LIBRARY_OVERRIDES;
        serde_json::from_str(overrides).expect("Failed to parse library patches")
    };
}

#[derive(Deserialize)]
pub struct LibraryPatch {
    downloads: Option<LibraryDownloads>,
    natives: Option<HashMap<String, String>>,
    rules: Option<Vec<Rule>>,
}

#[derive(Deserialize)]
pub struct LibraryPatches {
    #[serde(rename = "match")]
    match_: HashSet<String>,

    #[serde(rename = "override")]
    override_: Option<LibraryPatch>,

    #[serde(rename = "additionalLibraries")]
    additional_libraries: Option<Vec<Library>>,
}

lazy_static::lazy_static! {
    static ref LIBRARY_PATCHES: Vec<LibraryPatches> = {
        let overrides = build_config::MOJANG_LIBRARY_PATCHES;
        serde_json::from_str(overrides).expect("Failed to parse library overrides")
    };
}

lazy_static::lazy_static! {
    static ref LWJGL_VERSION_MATCHES: HashMap<String, String> = {
        let matches = build_config::LWJGL_VERSION_MATCHES;
        serde_json::from_str(matches).expect("Failed to parse lwjgl version matches")
    };
}

fn with_mojang_patches(libraries: Vec<&Library>) -> Vec<Library> {
    let mut result = vec![];
    for library in libraries {
        let mut library = library.clone();
        for override_ in &*LIBRARY_PATCHES {
            if override_.match_.contains(&library.get_full_name()) {
                if let Some(override_) = &override_.override_ {
                    info!("Modifying library: {}", library.get_full_name());
                    if let Some(downloads) = &override_.downloads {
                        library.downloads = Some(downloads.clone());
                    }
                    if let Some(natives) = &override_.natives {
                        library.natives = Some(natives.clone());
                    }
                    if let Some(rules) = &override_.rules {
                        library.rules = Some(rules.clone());
                    }
                }
                if let Some(additional_libraries) = &override_.additional_libraries {
                    info!(
                        "Adding additional libraries for {}",
                        library.get_full_name()
                    );
                    result.extend(additional_libraries.clone());
                }
            }
        }
        result.push(library.clone());
    }

    info!("Processed {} libraries with mojang overrides", result.len());

    result
}

pub fn with_overrides(libraries: Vec<&Library>, version_ids: &Vec<String>) -> Vec<Library> {
    let mut main_version = None;
    for version_id in version_ids {
        if let Some(version_match) = LWJGL_VERSION_MATCHES.get(version_id) {
            if main_version.is_some() {
                warn!("Multiple main lwjgl versions found");
            }
            info!("Found main lwjgl version: {}", version_match);
            main_version = Some(version_match.clone());
        }
    }
    if main_version.is_none() {
        info!("No main lwjgl version found");
    }

    let libraries = with_mojang_patches(libraries);

    let mut result = vec![];
    if let Some(main_version) = main_version {
        for library in libraries {
            if !LIBRARY_OVERRIDES
                .lwjgl_group_ids
                .contains(&library.get_group_id())
            {
                result.push(library.clone());
            }
        }

        for override_ in &LIBRARY_OVERRIDES.overrides {
            if override_.version == main_version {
                info!("Adding override libraries for version {}", main_version);
                result.extend(override_.libraries.clone());
            }
        }
    } else {
        result = libraries;
    }

    info!("Processed {} libraries with overrides", result.len());

    result
}
