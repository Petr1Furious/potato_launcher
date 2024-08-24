use reqwest::Client;
use serde::Deserialize;
use std::sync::mpsc::channel;
use std::{
    error::Error,
    sync::{Arc, Mutex},
};
use tokio::sync::{mpsc::unbounded_channel, oneshot};
use warp::Filter;
use warp::{http::Uri, reply::Reply};

use crate::message_provider::MessageProvider;
use crate::{
    config::build_config,
    lang::LangMessage,
};

use super::base::{AuthProvider, UserInfo};

pub const ELY_BY_BASE: &str = "https://ely.by/";

// TODO: test this horrible code
#[derive(Debug)]
struct InvalidCodeError;

impl std::error::Error for InvalidCodeError {}

impl std::fmt::Display for InvalidCodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid code error")
    }
}

pub struct ElyByAuthProvider {
    redirect_uri: Option<String>,
    token: Option<String>,
    message_provider: Arc<dyn MessageProvider>,
}

#[derive(Deserialize)]
struct AuthQuery {
    code: String,
}

impl ElyByAuthProvider {
    pub fn new(message_provider: Arc<dyn MessageProvider>) -> Self {
        Self {
            redirect_uri: None,
            token: None,
            message_provider,
        }
    }

    fn print_auth_url(&self) {
        if let Some(ref redirect_uri) = self.redirect_uri {
            let url = format!(
                "https://account.ely.by/oauth2/v1?client_id={}&redirect_uri={}&response_type=code&scope=account_info%20minecraft_server_session&prompt=select_account",
                build_config::get_elyby_client_id().unwrap(), redirect_uri
            );
            open::that(&url).unwrap();
            self.message_provider.set_message(LangMessage::AuthMessage { url });
        } else {
            panic!("redirect_uri is not set");
        }
    }

    async fn exchange_code(&self, code: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let client = Client::new();
        let resp = client
            .post("https://account.ely.by/api/oauth2/v1/token")
            .form(&[
                ("client_id", &build_config::get_elyby_client_id().unwrap()),
                (
                    "client_secret",
                    &build_config::get_elyby_client_secret().unwrap(),
                ),
                ("redirect_uri", self.redirect_uri.as_ref().unwrap()),
                ("grant_type", &"authorization_code".to_string()),
                ("code", &code.to_string()),
            ])
            .send()
            .await?;

        let status = resp.status();
        let data: serde_json::Value = resp.json().await?;
        if status != 200 {
            if data["error"] == "invalid_request" {
                return Err(Box::new(InvalidCodeError));
            }
        }

        if data["token_type"] != "Bearer" {
            return Err("Invalid token type".into());
        }

        Ok(data["access_token"].as_str().unwrap().to_string())
    }
}

impl AuthProvider for ElyByAuthProvider {
    async fn authenticate(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
        let (code_tx, mut code_rx) = unbounded_channel();
        let code_tx = Arc::new(Mutex::new(code_tx));

        let (error_tx, error_rx) = channel::<()>();
        let error_rx = Arc::new(Mutex::new(error_rx));

        let handle = {
            let code_tx = Arc::clone(&code_tx);
            warp::any()
                .and(warp::query::<AuthQuery>())
                .map(move |query: AuthQuery| {
                    let code_tx = code_tx.lock().unwrap();
                    code_tx.send(query.code.clone()).unwrap();

                    let error_rx = error_rx.lock().unwrap();
                    let error = error_rx.recv();
                    if error.is_ok() {
                        Box::new(warp::http::StatusCode::INTERNAL_SERVER_ERROR) as Box<dyn Reply>
                    } else {
                        Box::new(warp::redirect::temporary(
                            Uri::from_maybe_shared(format!(
                                "https://account.ely.by/oauth2/code/success?appName={}",
                                build_config::get_elyby_app_name().unwrap()
                            ))
                            .unwrap(),
                        ))
                    }
                })
        };

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let (addr, server) =
            warp::serve(handle).bind_with_graceful_shutdown(([127, 0, 0, 1], 0), async {
                shutdown_rx.await.ok();
            });

        self.redirect_uri = Some(format!("http://localhost:{}/", addr.port()));

        tokio::spawn(server);
        self.print_auth_url();

        loop {
            let code = code_rx.recv().await.unwrap();
            match self.exchange_code(&code).await {
                Ok(token) => {
                    self.token = Some(token);
                    break;
                }
                Err(e) => {
                    if e.downcast_ref::<InvalidCodeError>().is_none() {
                        shutdown_tx.send(()).unwrap();
                        return Err(e);
                    }
                    error_tx.send(()).unwrap();
                }
            }
            break;
        }

        shutdown_tx.send(()).unwrap();

        Ok(self.token.as_ref().unwrap().clone())
    }

    async fn get_user_info(&self, token: &str) -> Result<UserInfo, Box<dyn Error + Send + Sync>> {
        let client = Client::new();
        let resp = client
            .get("https://account.ely.by/api/account/v1/info")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?
            .error_for_status()?;

        let data: UserInfo = resp.json().await?;
        Ok(data)
    }
}
