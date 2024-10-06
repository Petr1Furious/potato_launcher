use std::path::{Path, PathBuf};

use shared::{
    files::CheckDownloadEntry,
    version::version_metadata::{Library, Os, Rule, VariableArgument},
};

fn get_os_name() -> String {
    if cfg!(windows) {
        "windows".to_string()
    } else if cfg!(target_os = "macos") {
        "osx".to_string()
    } else if cfg!(target_os = "linux") {
        "linux".to_string()
    } else {
        unimplemented!("Unsupported OS");
    }
}

fn get_system_arch() -> String {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "arm" => "arm32",
        arch => arch,
    }
    .to_string()
}

fn get_arch_os_name() -> String {
    get_os_name()
        + match get_system_arch().as_str() {
            "arm32" => "-arm32",
            "arm64" => "-arm64",
            _ => "",
        }
}

fn matches_current_os(rule: &Os) -> bool {
    let os_name = get_os_name();
    let os_arch = get_system_arch();

    if let Some(self_arch) = &rule.arch {
        if self_arch != &os_arch {
            return false;
        }
    }
    if let Some(self_name) = &rule.name {
        if self_name != &os_name && self_name != &format!("{}-{}", os_name, os_arch) {
            return false;
        }
    }

    true
}

fn rule_allowed_on_current_os(rule: &Rule) -> Option<bool> {
    let is_allowed = rule.action == "allow";
    let matching_features = vec!["has_custom_resolution"];

    let mut matches = true;

    if let Some(os) = &rule.os {
        if !matches_current_os(os) {
            matches = false;
        }
    }

    if let Some(features) = &rule.features {
        for (feature, value) in features {
            let contains = matching_features.contains(&feature.as_str());
            if contains != *value {
                matches = false;
                break;
            }
        }
    }

    if matches {
        Some(is_allowed)
    } else {
        None
    }
}

pub fn rules_apply(rules: &Vec<Rule>) -> bool {
    let mut some_allowed = false;
    for rule in rules {
        if let Some(is_allowed) = rule_allowed_on_current_os(rule) {
            if !is_allowed {
                return false;
            }
            some_allowed = true;
        }
    }
    some_allowed
}

pub fn get_matching_values(arg: &VariableArgument) -> Vec<&str> {
    match arg {
        VariableArgument::Simple(s) => vec![s],
        VariableArgument::Complex(complex) => {
            if rules_apply(&complex.rules) {
                complex.value.get_values()
            } else {
                vec![]
            }
        }
    }
}

fn get_natives_name(library: &Library) -> Option<&str> {
    let natives = library.natives.as_ref()?;
    // new versions have split natives, while all old ones are overwritten
    // so we only match the format in overwrites
    if let Some(natives_name) = natives.get(&get_arch_os_name()) {
        return Some(natives_name);
    }

    None
}

pub fn get_natives_path(library: &Library, libraries_dir: &Path) -> Option<PathBuf> {
    let natives_name = get_natives_name(library)?;
    let download = library.get_natives_download(&natives_name)?;
    let path = library.get_natives_path(natives_name, download, libraries_dir);
    Some(path)
}

pub fn get_check_download_entries(
    library: &Library,
    libraries_dir: &Path,
) -> Vec<CheckDownloadEntry> {
    let natives_name = get_natives_name(library);

    library.get_specific_check_download_entries(natives_name, libraries_dir)
}
