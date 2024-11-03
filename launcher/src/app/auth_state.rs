use image::Luma;
use qrcode::QrCode;
use shared::version::extra_version_metadata::AuthData;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use tokio::runtime::Runtime;

use crate::auth::auth::auth; // nice
use crate::auth::auth::AuthMessageProvider;
use crate::auth::base::{get_auth_provider, AuthProvider};
use crate::config::runtime_config::{self, Config, VersionAuthData};
use crate::lang::{Lang, LangMessage};
use crate::message_provider::MessageProvider as _;

use super::background_task::{BackgroundTask, BackgroundTaskResult};

#[derive(Clone, PartialEq)]
enum AuthStatus {
    NotAuthorized,
    Authorized(VersionAuthData),
    AuthorizeError(String),
    AuthorizeErrorOffline,
    AuthorizeErrorTimeout,
}

fn authenticate<Callback>(
    runtime: &Runtime,
    token: Option<&str>,
    auth_provider: Arc<dyn AuthProvider + Send + Sync>,
    auth_message_provider: Arc<AuthMessageProvider>,
    callback: Callback,
) -> BackgroundTask<AuthStatus>
where
    Callback: FnOnce() + Send + 'static,
{
    let token = token.map(|x| x.to_string());
    let fut = async move {
        match auth(token, auth_provider, auth_message_provider).await {
            Ok(auth_result) => AuthStatus::Authorized(VersionAuthData {
                token: auth_result.token,
                user_info: auth_result.user_info,
            }),

            Err(e) => {
                let mut connect_error = false;
                let mut timeout_error = false;
                if let Some(re) = e.downcast_ref::<reqwest::Error>() {
                    if re.is_connect() {
                        connect_error = true;
                    }
                    if re.is_timeout() || re.status().map(|s| s.as_u16()) == Some(524) {
                        timeout_error = true;
                    }
                }

                if connect_error {
                    AuthStatus::AuthorizeErrorOffline
                } else if timeout_error {
                    AuthStatus::AuthorizeErrorTimeout
                } else {
                    AuthStatus::AuthorizeError(e.to_string())
                }
            }
        }
    };

    BackgroundTask::with_callback(fut, runtime, Box::new(callback))
}

pub struct AuthState {
    auth_status: AuthStatus,
    auth_task: Option<BackgroundTask<AuthStatus>>,
    auth_message_provider: Arc<AuthMessageProvider>,
    runtime_auth: HashMap<String, VersionAuthData>,
}

impl AuthState {
    pub fn new(ctx: &egui::Context) -> Self {
        return AuthState {
            auth_status: AuthStatus::NotAuthorized,
            auth_task: None,
            auth_message_provider: Arc::new(AuthMessageProvider::new(ctx)),
            runtime_auth: HashMap::new(),
        };
    }

    pub fn update(&mut self, config: &mut runtime_config::Config, auth_data: &AuthData) -> bool {
        if let Some(task) = self.auth_task.as_ref() {
            if task.has_result() {
                self.auth_message_provider.clear();
                let task = self.auth_task.take().unwrap();
                let result = task.take_result();
                match result {
                    BackgroundTaskResult::Finished(auth_status) => match auth_status.clone() {
                        AuthStatus::Authorized(version_auth_data) => {
                            config
                                .versions_auth_data
                                .insert(auth_data.get_id(), version_auth_data.clone());
                            self.runtime_auth
                                .insert(auth_data.get_id(), version_auth_data);
                            config.save();

                            self.auth_status = auth_status;
                        }

                        _ => {
                            self.auth_status = auth_status;
                        }
                    },

                    BackgroundTaskResult::Cancelled => {
                        self.auth_status = AuthStatus::NotAuthorized;
                    }
                }

                return true;
            }
        }

        false
    }

    fn render_auth_window(auth_message: LangMessage, lang: Lang, ui: &mut egui::Ui) {
        egui::Window::new(LangMessage::Authorization.to_string(lang)).show(ui.ctx(), |ui| {
            ui.label(auth_message.to_string(lang));
            let url = match auth_message {
                LangMessage::AuthMessage { url } => Some(url),
                _ => None,
            }
            .unwrap();

            ui.hyperlink(&url);
            let code = QrCode::new(url).unwrap();
            let image = code.render::<Luma<u8>>().build();

            let mut png_bytes: Vec<u8> = Vec::new();
            let mut cursor = Cursor::new(&mut png_bytes);
            image::DynamicImage::ImageLuma8(image)
                .write_to(&mut cursor, image::ImageFormat::Png)
                .unwrap();

            let uri = "bytes://auth_qr.png";
            ui.ctx().include_bytes(uri, png_bytes.clone());
            ui.add(egui::Image::from_bytes(uri.to_string(), png_bytes));
        });
    }

    fn set_auth_task(
        &mut self,
        ctx: &egui::Context,
        runtime: &Runtime,
        auth_provider: Arc<dyn AuthProvider + Send + Sync>,
        token: Option<&str>,
    ) {
        let ctx = ctx.clone();
        let selected_token = token.map(|x| x.to_string());
        self.auth_message_provider = Arc::new(AuthMessageProvider::new(&ctx));
        self.auth_task = Some(authenticate(
            runtime,
            selected_token.as_deref(),
            auth_provider,
            self.auth_message_provider.clone(),
            move || {
                ctx.request_repaint();
            },
        ));
    }

    pub fn render_ui(
        &mut self,
        ui: &mut egui::Ui,
        config: &Config,
        runtime: &Runtime,
        ctx: &egui::Context,
        auth_data: &AuthData,
    ) {
        let lang = config.lang;
        let version_auth_data = config.get_version_auth_data(auth_data);
        let selected_username = version_auth_data.map(|x| x.user_info.username.clone());
        let selected_token = version_auth_data.map(|x| x.token.clone());

        let auth_provider = get_auth_provider(auth_data);
        let auth_provider_name = auth_provider.get_name();

        match &self.auth_status {
            AuthStatus::NotAuthorized if self.auth_task.is_none() => {
                if let Some(version_auth_data) = config.get_version_auth_data(auth_data) {
                    let token = version_auth_data.token.clone();
                    self.set_auth_task(ctx, runtime, auth_provider.clone(), Some(&token));
                }
            }
            _ => {}
        }

        match &self.auth_status {
            AuthStatus::NotAuthorized if self.auth_task.is_none() => {
                ui.label(LangMessage::AuthorizeUsing(auth_provider_name).to_string(lang));
            }
            AuthStatus::NotAuthorized => {
                ui.label(LangMessage::Authorizing.to_string(lang));
            }
            AuthStatus::AuthorizeError(e) => {
                ui.label(LangMessage::AuthError(e.clone()).to_string(lang));
            }
            AuthStatus::AuthorizeErrorOffline => {
                ui.label(
                    LangMessage::NoConnectionToAuthServer {
                        offline_username: selected_username.clone(),
                    }
                    .to_string(lang),
                );
            }
            AuthStatus::AuthorizeErrorTimeout => {
                ui.label(LangMessage::AuthTimeout.to_string(lang));
            }
            AuthStatus::Authorized(auth_data) => {
                ui.label(LangMessage::AuthorizedAs.to_string(lang));
                let text = egui::RichText::new(&auth_data.user_info.username)
                    .text_style(egui::TextStyle::Monospace);
                ui.label(text);
            }
        }

        if let Some(message) = self.auth_message_provider.get_message() {
            AuthState::render_auth_window(message, lang, ui);
        }

        match &self.auth_status {
            AuthStatus::Authorized(_) => {}
            AuthStatus::NotAuthorized if self.auth_task.is_some() => {}
            _ => {
                if ui.button(LangMessage::Authorize.to_string(lang)).clicked() {
                    self.set_auth_task(ctx, runtime, auth_provider, selected_token.as_deref());
                }
            }
        }
    }

    pub fn reset_auth_if_needed(&mut self, new_auth_data: &AuthData) {
        if !self.runtime_auth.contains_key(&new_auth_data.get_id()) {
            self.auth_status = AuthStatus::NotAuthorized;
            self.auth_task = None;
        }
    }

    pub fn ready_for_launch(&self, auth_data: &AuthData, config: &Config) -> bool {
        self.runtime_auth.contains_key(&auth_data.get_id())
            || self.auth_status == AuthStatus::AuthorizeErrorOffline
                && config.get_version_auth_data(auth_data).is_some()
    }

    pub fn online(&self) -> bool {
        match &self.auth_status {
            AuthStatus::Authorized(_) => true,
            _ => false,
        }
    }
}
