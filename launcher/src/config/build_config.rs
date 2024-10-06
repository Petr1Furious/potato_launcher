include!(concat!(env!("OUT_DIR"), "/generated.rs"));

pub fn get_launcher_name() -> String {
    LAUNCHER_NAME.to_string()
}

pub fn get_version_manifest_url() -> String {
    VERSION_MANIFEST_URL.to_string()
}

pub fn get_auto_update_base() -> Option<String> {
    AUTO_UPDATE_BASE.map(|url| url.to_string())
}

pub fn get_tgauth_base() -> Option<String> {
    TGAUTH_BASE.map(|base| base.to_string())
}

pub fn get_elyby_app_name() -> Option<String> {
    ELYBY_APP_NAME.map(|app_name| app_name.to_string())
}

pub fn get_elyby_client_id() -> Option<String> {
    ELYBY_CLIENT_ID.map(|client_id| client_id.to_string())
}

pub fn get_elyby_client_secret() -> Option<String> {
    ELYBY_CLIENT_SECRET.map(|client_secret| client_secret.to_string())
}

pub fn get_version() -> Option<String> {
    VERSION.map(|version| version.to_string())
}

pub fn get_display_launcher_name() -> String {
    match DISPLAY_LAUNCHER_NAME {
        Some(display_launcher_name) => display_launcher_name.to_string(),
        None => LAUNCHER_NAME.to_string(),
    }
}

pub const LAUNCHER_ICON: &[u8] = include_bytes!("../../assets/potato_launcher.png");

pub const LIBRARY_OVERRIDES: &str = include_str!("../../meta/library-overrides.json");

pub const MOJANG_LIBRARY_PATCHES: &str = include_str!("../../meta/mojang-library-patches.json");

pub const LWJGL_VERSION_MATCHES: &str = include_str!("../../meta/lwjgl-version-matches.json");
