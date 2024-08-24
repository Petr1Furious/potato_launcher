use serde::{Deserialize, Serialize};
use std::{error::Error, sync::Arc};

use super::{elyby::ElyByAuthProvider, telegram::TGAuthProvider};
use crate::{config::build_config, message_provider::MessageProvider};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct UserInfo {
    pub uuid: String,
    pub username: String,
}

pub trait AuthProvider {
    async fn authenticate(&mut self) -> Result<String, Box<dyn Error + Send + Sync>>;
    async fn get_user_info(&self, token: &str) -> Result<UserInfo, Box<dyn Error + Send + Sync>>;
}

pub enum AuthProviderEnum {
    TG(TGAuthProvider),
    ElyBy(ElyByAuthProvider),
}

impl AuthProvider for AuthProviderEnum {
    async fn authenticate(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
        match self {
            AuthProviderEnum::TG(provider) => provider.authenticate().await,
            AuthProviderEnum::ElyBy(provider) => provider.authenticate().await,
        }
    }

    async fn get_user_info(&self, token: &str) -> Result<UserInfo, Box<dyn Error + Send + Sync>> {
        match self {
            AuthProviderEnum::TG(provider) => provider.get_user_info(token).await,
            AuthProviderEnum::ElyBy(provider) => provider.get_user_info(token).await,
        }
    }
}

pub fn get_auth_provider(message_provider: Arc<dyn MessageProvider>) -> AuthProviderEnum {
    if let Some(base_url) = build_config::get_tgauth_base() {
        AuthProviderEnum::TG(TGAuthProvider::new(base_url, message_provider))
    } else if let Some(_app_name) = build_config::get_elyby_app_name() {
        AuthProviderEnum::ElyBy(ElyByAuthProvider::new(message_provider))
    } else {
        panic!("No auth provider is set");
    }
}
