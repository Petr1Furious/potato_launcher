use crate::config::build_config;
use crate::lang;
use crate::config::runtime_config::Config;
use std::fs;
use std::path::PathBuf;

use colored::Colorize;

pub fn set_sigint_handler(config: &Config) {
    let lang = config.lang.clone();

    ctrlc::set_handler(move || {
        println!("{}", lang::get_loc(&lang).exiting.red());
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");
}

pub fn print_error_and_exit(message: &str) -> ! {
    println!("{}", message.red());
    std::process::exit(1);
}

pub fn get_temp_dir() -> PathBuf {
    let temp_dir = std::env::temp_dir();
    let temp_dir = temp_dir.join(build_config::get_launcher_name());
    if !temp_dir.exists() {
        fs::create_dir_all(&temp_dir).unwrap();
    }
    temp_dir
}
