use crate::lang::LangMessage;
use crate::message_provider::MessageProvider;

use super::base::{AuthProvider, UserInfo};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::Arc;
use std::{collections::HashMap, time::Duration};

#[derive(Deserialize, Serialize)]
struct LoginStartResponse {
    code: String,
    intermediate_token: String,
}

#[derive(Deserialize, Serialize)]
struct BotInfo {
    bot_username: String,
}

pub struct TGAuthProvider {
    client: Client,
    base_url: String,
    bot_name: Option<String>,
    message_provider: Arc<dyn MessageProvider>,
}

impl TGAuthProvider {
    pub fn new(base_url: String, message_provider: Arc<dyn MessageProvider>) -> Self {
        TGAuthProvider {
            client: Client::new(),
            base_url,
            bot_name: None,
            message_provider,
        }
    }

    async fn get_bot_name(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
        if self.bot_name.is_none() {
            let body = self
                .client
                .get(format!("{}/info", self.base_url))
                .send()
                .await?
                .text()
                .await?;

            let bot_info: BotInfo = serde_json::from_str(&body).unwrap();
            self.bot_name = Some(bot_info.bot_username);
        }
        Ok(self.bot_name.clone().unwrap())
    }
}

impl AuthProvider for TGAuthProvider {
    async fn authenticate(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
        let bot_name = self.get_bot_name().await?;
        let body = self
            .client
            .post(format!("{}/login/start", self.base_url))
            .send()
            .await?
            .text()
            .await?;
        let start_resp: LoginStartResponse = serde_json::from_str(&body).unwrap();

        let tg_deeplink = format!("https://t.me/{}?start={}", bot_name, start_resp.code);
        open::that(&tg_deeplink).unwrap();
        self.message_provider
            .set_message(LangMessage::AuthMessage { url: tg_deeplink });

        let access_token;
        loop {
            let response = self
                .client
                .post(format!("{}/login/poll", self.base_url))
                .json(&serde_json::json!({
                    "intermediate_token": start_resp.intermediate_token
                }))
                .send()
                .await;

            match response {
                Ok(resp) => {
                    resp.error_for_status_ref()?;

                    let body = resp.text().await?;
                    let poll_resp: HashMap<String, serde_json::Value> =
                        serde_json::from_str(&body).unwrap();

                    access_token = poll_resp
                        .get("user")
                        .unwrap()
                        .get("access_token")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string();
                    break;
                }
                Err(e) => {
                    if !e.is_timeout() {
                        return Err(Box::new(e));
                    }
                }
            }

            std::thread::sleep(Duration::from_secs(1));
        }

        Ok(access_token)
    }

    async fn get_user_info(&self, token: &str) -> Result<UserInfo, Box<dyn Error + Send + Sync>> {
        let resp = self
            .client
            .get(format!("{}/login/profile", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        resp.error_for_status_ref()?;

        let body = resp.text().await?;
        let user_info: UserInfo = serde_json::from_str(&body).unwrap();
        Ok(user_info)
    }
}
