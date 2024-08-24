use crate::config::build_config;
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
