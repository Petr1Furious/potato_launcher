use super::base::{AuthProvider, AuthResultData, AuthState};
use super::version_auth_data::UserInfo;
use crate::lang::LangMessage;
use crate::message_provider::MessageProvider;
use crate::vendor::minecraft_msa_auth::MinecraftAuthorizationFlow;
use async_trait::async_trait;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, ClientId, DeviceAuthorizationUrl, DeviceCodeErrorResponseType, RefreshToken,
    RequestTokenError, Scope, StandardDeviceAuthorizationResponse, TokenResponse, TokenUrl,
};
use reqwest::{Client, Url};
use serde::Deserialize;
use std::time::Duration;

const MSA_DEVICE_CODE_URL: &str = "https://login.live.com/oauth20_connect.srf";
const MSA_TOKEN_URL: &str = "https://login.live.com/oauth20_token.srf";
const MSA_CLIENT_ID: &str = "00000000441cc96b";
const MSA_SCOPE: &str = "service::user.auth.xboxlive.com::MBI_SSL";

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Timeout during authentication")]
    AuthTimeout,
}

pub struct MicrosoftAuthProvider {}

#[derive(Deserialize)]
struct MinecraftProfileResponse {
    id: String,
    name: String,
}

fn get_oauth_client() -> oauth2::basic::BasicClient {
    oauth2::basic::BasicClient::new(
        ClientId::new(MSA_CLIENT_ID.to_string()),
        None,
        AuthUrl::new(MSA_DEVICE_CODE_URL.to_string()).unwrap(),
        Some(TokenUrl::new(MSA_TOKEN_URL.to_string()).unwrap()),
    )
    .set_device_authorization_url(
        DeviceAuthorizationUrl::new(MSA_DEVICE_CODE_URL.to_string()).unwrap(),
    )
}

async fn get_ms_token(message_provider: &dyn MessageProvider) -> anyhow::Result<AuthResultData> {
    let client = get_oauth_client();

    let details: StandardDeviceAuthorizationResponse = client
        .exchange_device_code()?
        .add_scope(Scope::new(MSA_SCOPE.to_string()))
        .add_extra_param("response_type", "device_code")
        .request_async(async_http_client)
        .await?;

    let code = details.user_code().secret().to_string();
    let url =
        Url::parse_with_params(details.verification_uri(), &[("otc", code.clone())])?.to_string();

    let _ = open::that(&url);
    message_provider.set_message(LangMessage::DeviceAuthMessage { url, code });

    let token = client
        .exchange_device_access_token(&details)
        .request_async(
            async_http_client,
            tokio::time::sleep,
            Some(Duration::from_secs(60 * 5)),
        )
        .await
        .map_err(|e| -> anyhow::Error {
            match &e {
                RequestTokenError::ServerResponse(resp)
                    if *resp.error() == DeviceCodeErrorResponseType::ExpiredToken =>
                {
                    AuthError::AuthTimeout.into()
                }
                _ => e.into(),
            }
        })?;

    Ok(AuthResultData {
        access_token: token.access_token().secret().to_string(),
        refresh_token: token.refresh_token().map(|t| t.secret().to_string()),
    })
}

impl MicrosoftAuthProvider {
    pub fn new() -> Self {
        MicrosoftAuthProvider {}
    }
}

#[async_trait]
impl AuthProvider for MicrosoftAuthProvider {
    async fn authenticate(&self, message_provider: &dyn MessageProvider) -> anyhow::Result<AuthState> {
        let ms_token = get_ms_token(message_provider).await?;
        message_provider.clear();
        let mc_flow = MinecraftAuthorizationFlow::new(Client::new());
        let mc_token = mc_flow
            .exchange_microsoft_token(ms_token.access_token)
            .await?
            .access_token()
            .clone()
            .0;

        Ok(AuthState::UserInfo(AuthResultData {
            access_token: mc_token,
            refresh_token: Some(ms_token.refresh_token.unwrap()),
        }))
    }

    async fn refresh(&self, refresh_token: String) -> anyhow::Result<AuthState> {
        let oauth_client = get_oauth_client();
        let token_response = oauth_client
            .exchange_refresh_token(&RefreshToken::new(refresh_token))
            .request_async(async_http_client)
            .await?;

        Ok(AuthState::UserInfo(AuthResultData {
            access_token: token_response.access_token().secret().to_string(),
            refresh_token: token_response
                .refresh_token()
                .map(|t| t.secret().to_string()),
        }))
    }

    async fn get_user_info(&self, token: &str) -> anyhow::Result<AuthState> {
        let client = Client::new();
        let resp: MinecraftProfileResponse = client
            .get("https://api.minecraftservices.com/minecraft/profile")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(AuthState::Success(UserInfo {
            uuid: resp.id,
            username: resp.name,
        }))
    }

    fn get_auth_url(&self) -> Option<String> {
        None
    }

    fn get_name(&self) -> String {
        "Microsoft".to_string()
    }
}
