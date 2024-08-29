use crate::config::build_config;
use crate::constants;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

pub fn set_sigint_handler() {
    ctrlc::set_handler(move || {
        println!("Exiting...");
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");
}

pub fn get_temp_dir() -> PathBuf {
    let temp_dir = std::env::temp_dir();
    let temp_dir = temp_dir.join(build_config::get_launcher_name());
    if !temp_dir.exists() {
        fs::create_dir_all(&temp_dir).unwrap();
    }
    temp_dir
}

pub fn is_read_only_error(e: &Box<dyn Error>) -> bool {
    if let Some(e) = e.downcast_ref::<std::io::Error>() {
        return e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(18);
    }
    false
}

pub fn validate_xmx(xmx: &str) -> bool {
    let xmx = xmx.trim();
    if xmx.is_empty() {
        return false;
    }

    let xmx = xmx.to_uppercase();
    if xmx.ends_with("M") {
        if let Ok(xmx) = xmx[..xmx.len() - 1].parse::<u32>() {
            return xmx >= constants::MIN_JAVA_MB && xmx <= constants::MAX_JAVA_MB;
        }
    } else if xmx.ends_with("G") {
        if let Ok(xmx) = xmx[..xmx.len() - 1].parse::<u32>() {
            return xmx >= constants::MIN_JAVA_MB * 1024 && xmx <= constants::MAX_JAVA_MB * 1024;
        }
    }

    return false;
}
