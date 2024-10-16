use std::env;
use std::fs;

fn main() {
    let build_envs = ["LAUNCHER_NAME", "VERSION_MANIFEST_URL"];

    let optional_envs = ["AUTO_UPDATE_BASE", "VERSION"];

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = format!("{}/generated.rs", out_dir);

    let mut config_content = String::new();
    for env in build_envs.iter() {
        let value = env::var(env).expect(&format!("{} is not set", env));
        config_content.push_str(&format!("pub const {}: &str = \"{}\";\n", env, value));
    }
    for env in optional_envs.iter() {
        match env::var(env) {
            Ok(value) => {
                config_content.push_str(&format!(
                    "pub const {}: Option<&str> = Some(\"{}\");\n",
                    env, value
                ));
            }
            Err(_) => {
                config_content.push_str(&format!("pub const {}: Option<&str> = None;\n", env));
            }
        }
    }
    fs::write(dest_path, config_content).unwrap();

    if cfg!(target_os = "windows") {
        let data_launcher_name = env::var("LAUNCHER_NAME")
            .unwrap()
            .to_lowercase()
            .replace(" ", "_");
        let mut res = winres::WindowsResource::new();
        res.set_icon(&format!("assets/{}.ico", data_launcher_name));
        res.compile().unwrap();
    }
}
