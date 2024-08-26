include!(concat!(env!("OUT_DIR"), "/generated.rs"));

pub fn get_launcher_name() -> String {
    LAUNCHER_NAME.to_string()
}

pub fn get_server_base() -> String {
    SERVER_BASE.to_string()
}

pub fn get_tgauth_base() -> Option<String> {
    match TGAUTH_BASE {
        Some(base) => Some(base.to_string()),
        None => None,
    }
}

pub fn get_elyby_app_name() -> Option<String> {
    match ELYBY_APP_NAME {
        Some(app_name) => Some(app_name.to_string()),
        None => None,
    }
}

pub fn get_elyby_client_id() -> Option<String> {
    match ELYBY_CLIENT_ID {
        Some(client_id) => Some(client_id.to_string()),
        None => None,
    }
}

pub fn get_elyby_client_secret() -> Option<String> {
    match ELYBY_CLIENT_SECRET {
        Some(client_secret) => Some(client_secret.to_string()),
        None => None,
    }
}

pub fn get_version() -> Option<String> {
    match VERSION {
        Some(version) => Some(version.to_string()),
        None => None,
    }
}
