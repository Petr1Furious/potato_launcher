use std::error::Error;
use std::sync::Arc;

use super::base::AuthProvider;
use super::base::{self, UserInfo};

use crate::message_provider::MessageProvider;

pub struct AuthResult {
    pub token: String,
    pub user_info: UserInfo,
}

pub async fn auth(
    existing_token: Option<String>,
    message_provider: Arc<dyn MessageProvider>,
) -> Result<AuthResult, Box<dyn Error + Send + Sync>> {
    let mut token = existing_token;
    let mut user_info: Option<UserInfo> = None;

    let mut auth_provider = base::get_auth_provider(message_provider);

    let tries_num = if token.is_some() { 2 } else { 1 };
    for i in 0..tries_num {
        if token.is_none() {
            token = Some(auth_provider.authenticate().await?);
        }

        match auth_provider.get_user_info(token.as_ref().unwrap()).await {
            Ok(info) => {
                user_info = Some(info);
                break;
            }

            Err(e) => {
                let mut token_error = false;
                if let Some(re) = e.downcast_ref::<reqwest::Error>() {
                    if let Some(status) = re.status() {
                        if status.is_client_error() {
                            token_error = true;
                        }
                    }
                }

                if token_error && i + 1 != tries_num {
                    // try again with a new token in the next iteration
                    token = None;
                } else {
                    return Err(e);
                }
            }
        }
    }

    return Ok(AuthResult {
        token: token.unwrap(),
        user_info: user_info.unwrap(),
    });
}
