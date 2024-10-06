use image::Luma;
use qrcode::QrCode;
use std::io::Cursor;
use std::sync::{mpsc, Arc, Mutex};
use tokio::runtime::Runtime;

use crate::auth::{auth::auth, base::UserInfo};
use crate::config::runtime_config;
use crate::lang::{Lang, LangMessage};
use crate::message_provider::MessageProvider;

use super::task::Task;

#[derive(Clone, PartialEq)]
enum AuthStatus {
    Authorizing,
    Authorized,
    AuthorizeError(String),
    AuthorizeErrorOffline,
    AuthorizeErrorTimeout,
}

struct AuthResult {
    status: AuthStatus,
    token: Option<String>,
    user_info: Option<UserInfo>,
}

struct AuthMessageState {
    auth_message: Option<LangMessage>,
}

struct AuthMessageProvider {
    state: Arc<Mutex<AuthMessageState>>,
    ctx: egui::Context,
}

impl AuthMessageProvider {
    fn new(ctx: &egui::Context) -> Self {
        Self {
            state: Arc::new(Mutex::new(AuthMessageState { auth_message: None })),
            ctx: ctx.clone(),
        }
    }
}

impl MessageProvider for AuthMessageProvider {
    fn set_message(&self, message: LangMessage) {
        if let LangMessage::AuthMessage { .. } = message {
            let mut state = self.state.lock().unwrap();
            state.auth_message = Some(message);
            self.ctx.request_repaint();
        } else {
            panic!("Expected AuthMessage, got {:?}", message);
        }
    }

    fn get_message(&self) -> Option<LangMessage> {
        let state = self.state.lock().unwrap();
        return state.auth_message.clone();
    }

    fn clear(&self) {
        let mut state = self.state.lock().unwrap();
        state.auth_message = None;
        self.ctx.request_repaint();
    }
}

fn authenticate(
    runtime: &Runtime,
    token: Option<String>,
    message_provider: Arc<AuthMessageProvider>,
) -> Task<AuthResult> {
    let (tx, rx) = mpsc::channel();

    runtime.spawn(async move {
        let result = match auth(token, message_provider.clone()).await {
            Ok(auth_result) => AuthResult {
                status: AuthStatus::Authorized,
                token: Some(auth_result.token),
                user_info: Some(auth_result.user_info),
            },

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

                AuthResult {
                    status: if connect_error {
                        AuthStatus::AuthorizeErrorOffline
                    } else if timeout_error {
                        AuthStatus::AuthorizeErrorTimeout
                    } else {
                        AuthStatus::AuthorizeError(e.to_string())
                    },
                    token: None,
                    user_info: None,
                }
            }
        };

        let _ = tx.send(result);
        message_provider.ctx.request_repaint();
    });

    return Task::new(rx);
}

pub struct AuthState {
    auth_status: AuthStatus,
    auth_task: Option<Task<AuthResult>>,
    auth_message_provider: Arc<AuthMessageProvider>,
}

impl AuthState {
    pub fn new(ctx: &egui::Context) -> Self {
        let auth_message_provider = Arc::new(AuthMessageProvider::new(ctx));

        return AuthState {
            auth_status: AuthStatus::Authorizing,
            auth_task: None,
            auth_message_provider,
        };
    }

    pub fn update(&mut self, runtime: &Runtime, config: &mut runtime_config::Config) {
        if self.auth_status == AuthStatus::Authorizing && !self.auth_task.is_some() {
            self.auth_message_provider.clear();
            self.auth_task = Some(authenticate(
                runtime,
                config.token.clone(),
                self.auth_message_provider.clone(),
            ));
        }

        if let Some(task) = self.auth_task.as_ref() {
            if let Some(result) = task.take_result() {
                match &result.status {
                    AuthStatus::Authorized => {
                        let mut changed = false;
                        if config.token != result.token {
                            config.token = result.token;
                            changed = true;
                        }
                        if config.user_info != result.user_info {
                            config.user_info = result.user_info;
                            changed = true;
                        }
                        if changed {
                            runtime_config::save_config(config);
                        }
                    }

                    _ => {}
                }
                self.auth_status = result.status;
                self.auth_task = None;
            }
        }
    }

    pub fn render_ui(&mut self, ui: &mut egui::Ui, lang: &Lang, config_username: Option<&str>) {
        ui.label(match &self.auth_status {
            AuthStatus::Authorizing => LangMessage::Authorizing.to_string(lang),
            AuthStatus::AuthorizeError(e) => LangMessage::AuthError(e.clone()).to_string(lang),
            AuthStatus::AuthorizeErrorOffline => LangMessage::NoConnectionToAuthServer {
                offline_username: config_username.map(|username| username.to_string()),
            }
            .to_string(lang),
            AuthStatus::AuthorizeErrorTimeout => LangMessage::AuthTimeout.to_string(lang),
            AuthStatus::Authorized => LangMessage::AuthorizedAs(
                config_username
                    .expect("Authorized, but no username")
                    .to_string(),
            )
            .to_string(lang),
        });

        if self.auth_task.is_some() {
            let message = self.auth_message_provider.get_message();
            if let Some(message) = message {
                egui::Window::new(LangMessage::Authorization.to_string(lang)).show(
                    ui.ctx(),
                    |ui| {
                        ui.label(message.to_string(lang));
                        let url = match message {
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
                    },
                );
            }
        }

        if self.auth_status != AuthStatus::Authorized && self.auth_status != AuthStatus::Authorizing
        {
            if ui.button(LangMessage::Authorize.to_string(lang)).clicked() {
                self.auth_status = AuthStatus::Authorizing;
            }
        }
    }

    pub fn ready_for_launch(config: &runtime_config::Config) -> bool {
        config.user_info.is_some()
    }

    pub fn online(&self) -> bool {
        self.auth_status == AuthStatus::Authorized
    }
}
