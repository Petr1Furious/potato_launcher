mod generate;
mod progress;
mod spec;
mod utils;

use clap::{Arg, Command};
use log::error;
use spec::VersionsSpec;
use std::{path::PathBuf, process::exit};
use tokio::runtime::Runtime;

fn parse_path(v: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(v);
    if path.exists() {
        Ok(path)
    } else {
        Err(String::from("The specified file does not exist"))
    }
}

fn main() {
    env_logger::init();

    let matches = Command::new("generate-modpack")
        .about("Generates modpacks based on a specification file")
        .arg(
            Arg::new("spec_file")
                .help("Path to the specification file")
                .required(true)
                .short('s')
                .value_parser(parse_path),
        )
        .arg(
            Arg::new("output_dir")
                .help("Output directory")
                .default_value("./generated"),
        )
        .arg(
            Arg::new("work_dir")
                .help("Working directory")
                .default_value("./workdir"),
        )
        .get_matches();

    let spec_file = matches.get_one::<PathBuf>("spec_file").unwrap();
    let output_dir = matches.get_one::<String>("output_dir").unwrap();
    let output_dir = PathBuf::from(output_dir);
    let work_dir = matches.get_one::<String>("work_dir").unwrap();
    let work_dir = PathBuf::from(work_dir);

    let spec_file_path = spec_file.clone();
    let output_dir_path = output_dir.clone();
    let work_dir_path = work_dir.clone();

    let rt = Runtime::new().unwrap();
    let spec = rt.block_on(VersionsSpec::from_file(&spec_file_path));
    match spec {
        Ok(spec) => {
            rt.block_on(spec.generate(&output_dir_path, &work_dir_path))
                .unwrap();
        }
        Err(e) => {
            error!("Failed to read spec file: {}", e);
            exit(1);
        }
    }
}
