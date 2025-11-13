include!(concat!(env!("OUT_DIR"), "/generated.rs"));

pub fn get_launcher_name() -> String {
    LAUNCHER_NAME.to_string()
}

pub fn get_data_launcher_name() -> String {
    LAUNCHER_NAME.to_string().to_lowercase().replace(" ", "_")
}

pub fn get_default_version_manifest_url() -> String {
    VERSION_MANIFEST_URL.to_string()
}

pub fn get_auto_update_base() -> Option<String> {
    AUTO_UPDATE_BASE.map(|url| url.to_string())
}

pub fn get_version() -> Option<String> {
    VERSION.map(|version| version.to_string())
}

pub const LIBRARY_OVERRIDES: &str = include_str!("../../meta/library-overrides.json");

pub const MOJANG_LIBRARY_PATCHES: &str = include_str!("../../meta/mojang-library-patches.json");

pub const LWJGL_VERSION_MATCHES: &str = include_str!("../../meta/lwjgl-version-matches.json");

include!(concat!(env!("OUT_DIR"), "/icon_file_bytes.rs"));
