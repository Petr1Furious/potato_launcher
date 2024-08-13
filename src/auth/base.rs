use reqwest::Error;
use serde::{Serialize, Deserialize};

use crate::config::build_config;
use super::telegram::TGAuthProvider;

#[derive(Serialize, Deserialize)]
pub struct UserInfo {
    pub uuid: String,
    pub username: String,
}

pub trait AuthProvider {
    async fn authenticate(&mut self) -> Result<String, Error>;
    async fn get_user_info(&self, token: &String) -> Result<UserInfo, Error>;
    fn as_any(&self) -> &dyn std::any::Any;
}

pub fn get_auth_provider() -> impl AuthProvider {
    if let Some(base_url) = build_config::get_tgauth_base() {
        return TGAuthProvider::new(base_url);
    } else if let Some(_app_name) = build_config::ELYBY_APP_NAME {
        unimplemented!()
    } else {
        panic!("No auth provider is set");
    }
}
