use serde::{Serialize, Deserialize};
use std::error::Error;

use crate::{config::build_config, lang::Lang};
use super::{elyby::ElyByAuthProvider, telegram::TGAuthProvider};

#[derive(Serialize, Deserialize)]
pub struct UserInfo {
    pub uuid: String,
    pub username: String,
}

pub trait AuthProvider {
    async fn authenticate(&mut self) -> Result<String, Box<dyn Error>>;
    async fn get_user_info(&self, token: &str) -> Result<UserInfo, Box<dyn Error>>;
    fn as_any(&self) -> &dyn std::any::Any;
}

pub enum AuthProviderEnum {
    TG(TGAuthProvider),
    ElyBy(ElyByAuthProvider),
}

impl AuthProvider for AuthProviderEnum {
    async fn authenticate(&mut self) -> Result<String, Box<dyn Error>> {
        match self {
            AuthProviderEnum::TG(provider) => provider.authenticate().await,
            AuthProviderEnum::ElyBy(provider) => provider.authenticate().await,
        }
    }

    async fn get_user_info(&self, token: &str) -> Result<UserInfo, Box<dyn Error>> {
        match self {
            AuthProviderEnum::TG(provider) => provider.get_user_info(token).await,
            AuthProviderEnum::ElyBy(provider) => provider.get_user_info(token).await,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        match self {
            AuthProviderEnum::TG(provider) => provider,
            AuthProviderEnum::ElyBy(provider) => provider,
        }
    }
}

pub fn get_auth_provider(lang: Lang) -> AuthProviderEnum {
    if let Some(base_url) = build_config::get_tgauth_base() {
        AuthProviderEnum::TG(TGAuthProvider::new(base_url))
    } else if let Some(_app_name) = build_config::ELYBY_APP_NAME {
        AuthProviderEnum::ElyBy(ElyByAuthProvider::new(lang))
    } else {
        panic!("No auth provider is set");
    }
}
