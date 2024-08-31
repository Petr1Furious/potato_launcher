use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum Lang {
    English,
    Russian,
}

#[derive(Clone, PartialEq, Debug)]
pub enum LangMessage {
    AuthMessage { url: String },
    NoConnectionToAuthServer { offline_username: Option<String> },
    AuthTimeout,
    AuthError(String),
    AuthorizedAs(String),
    Authorizing,
    Authorize,
    FetchingModpackIndexes,
    FetchedRemoteIndexes,
    NoConnectionToIndexServer,
    ErrorFetchingRemoteIndexes(String),
    FetchIndexes,
    SelectModpack,
    NoIndexes,
    CheckingFiles,
    DownloadingFiles,
    SyncModpack,
    ModpackNotSynced,
    SyncingModpack,
    ModpackSynced,
    ModpackSyncError(String),
    DownloadingJava,
    DownloadJava { version: String },
    JavaInstalled { version: String },
    NeedJava { version: String },
    ErrorDownloadingJava(String),
    NoConnectionToJavaServer,
    JavaSettings,
    SelectedJavaPath { path: Option<String> },
    JavaXMX,
    SelectJavaPath,
    Launch,
    LaunchError(String),
    Running,
    Language,
    LanguageName,
    DownloadingUpdate,
    CheckingForUpdates,
    Launching,
    ErrorCheckingForUpdates(String),
    ErrorDownloadingUpdate(String),
    ErrorReadOnly,
    ProceedToLauncher,
    Authorization,
    Modpacks,
    ForceOverwrite,
    ForceOverwriteWarning,
    OpenLauncherDirectory,
    KillMinecraft,
    CloseLauncherAfterLaunch,
    DownloadAndLaunch,
    CancelLaunch,
}

impl LangMessage {
    pub fn to_string(&self, lang: &Lang) -> String {
        match self {
            LangMessage::AuthMessage { url: _ } => match lang {
                Lang::English => {
                    "Authorize in the browser window.\nOr open the link manually.".to_string()
                }
                Lang::Russian => {
                    "Авторизуйтесь в открывшемся окне браузера.\nИли откройте ссылку вручную."
                        .to_string()
                }
            },
            LangMessage::NoConnectionToAuthServer {
                offline_username: username,
            } => match lang {
                Lang::English => format!(
                    "No connection to the authorization server.\nOffline username: {}",
                    username.as_ref().unwrap_or(&"None".to_string())
                ),
                Lang::Russian => format!(
                    "Нет подключения к серверу авторизации.\nОфлайн имя пользователя: {}",
                    username.as_ref().unwrap_or(&"Отсутствует".to_string())
                ),
            },
            LangMessage::AuthTimeout => match lang {
                Lang::English => "Authorization timeout".to_string(),
                Lang::Russian => "Превышено время авторизации".to_string(),
            },
            LangMessage::AuthError(e) => match lang {
                Lang::English => format!("Authorization error: {}", e),
                Lang::Russian => format!("Ошибка авторизации: {}", e),
            },
            LangMessage::AuthorizedAs(username) => match lang {
                Lang::English => format!("Authorized as {}", username),
                Lang::Russian => format!("Авторизован как {}", username),
            },
            LangMessage::Authorizing => match lang {
                Lang::English => "Authorizing...".to_string(),
                Lang::Russian => "Авторизация...".to_string(),
            },
            LangMessage::Authorize => match lang {
                Lang::English => "Authorize".to_string(),
                Lang::Russian => "Авторизоваться".to_string(),
            },
            LangMessage::FetchingModpackIndexes => match lang {
                Lang::English => "Fetching modpack list...".to_string(),
                Lang::Russian => "Получение списка модпаков...".to_string(),
            },
            LangMessage::FetchedRemoteIndexes => match lang {
                Lang::English => "Modpack list fetched".to_string(),
                Lang::Russian => "Список модпаков получен".to_string(),
            },
            LangMessage::NoConnectionToIndexServer => match lang {
                Lang::English => "No connection to the modpack server".to_string(),
                Lang::Russian => "Нет подключения к серверу модпаков".to_string(),
            },
            LangMessage::ErrorFetchingRemoteIndexes(s) => match lang {
                Lang::English => format!("Error fetching remote modpack list: {}", s),
                Lang::Russian => format!("Ошибка получения списка модпаков с сервера: {}", s),
            },
            LangMessage::FetchIndexes => match lang {
                Lang::English => "Fetch modpack list".to_string(),
                Lang::Russian => "Получить список модпаков".to_string(),
            },
            LangMessage::SelectModpack => match lang {
                Lang::English => "Select modpack".to_string(),
                Lang::Russian => "Выберите модпак".to_string(),
            },
            LangMessage::NoIndexes => match lang {
                Lang::English => "No modpacks fetched".to_string(),
                Lang::Russian => "Список модпаков пуст".to_string(),
            },
            LangMessage::CheckingFiles => match lang {
                Lang::English => "Checking files...".to_string(),
                Lang::Russian => "Проверка файлов...".to_string(),
            },
            LangMessage::DownloadingFiles => match lang {
                Lang::English => "Downloading files...".to_string(),
                Lang::Russian => "Загрузка файлов...".to_string(),
            },
            LangMessage::SyncModpack => match lang {
                Lang::English => "Sync modpack".to_string(),
                Lang::Russian => "Синхронизировать модпак".to_string(),
            },
            LangMessage::ModpackNotSynced => match lang {
                Lang::English => "Modpack not synced".to_string(),
                Lang::Russian => "Модпак не синхронизирован".to_string(),
            },
            LangMessage::SyncingModpack => match lang {
                Lang::English => "Syncing modpack...".to_string(),
                Lang::Russian => "Синхронизация модпака...".to_string(),
            },
            LangMessage::ModpackSynced => match lang {
                Lang::English => "Modpack up-to-date".to_string(),
                Lang::Russian => "Модпак синхронизирован".to_string(),
            },
            LangMessage::ModpackSyncError(e) => match lang {
                Lang::English => format!("Error syncing modpack: {}", e),
                Lang::Russian => format!("Ошибка синхронизации модпака: {}", e),
            },
            LangMessage::DownloadingJava => match lang {
                Lang::English => "Downloading Java...".to_string(),
                Lang::Russian => "Загрузка Java...".to_string(),
            },
            LangMessage::DownloadJava { version } => match lang {
                Lang::English => format!("Download Java {}", version),
                Lang::Russian => format!("Загрузить Java {}", version),
            },
            LangMessage::JavaInstalled { version } => match lang {
                Lang::English => format!("Java {} installed", version),
                Lang::Russian => format!("Java {} установлена", version),
            },
            LangMessage::NeedJava { version } => match lang {
                Lang::English => format!("Java {} not installed", version),
                Lang::Russian => format!("Java {} не установлена", version),
            },
            LangMessage::ErrorDownloadingJava(e) => match lang {
                Lang::English => format!("Error downloading Java: {}", e),
                Lang::Russian => format!("Ошибка загрузки Java: {}", e),
            },
            LangMessage::NoConnectionToJavaServer => match lang {
                Lang::English => "No connection to the Java download server".to_string(),
                Lang::Russian => "Нет подключения к серверу загрузки Java".to_string(),
            },
            LangMessage::JavaSettings => match lang {
                Lang::English => "Java Settings".to_string(),
                Lang::Russian => "Настройки Java".to_string(),
            },
            LangMessage::SelectedJavaPath { path } => match lang {
                Lang::English => format!(
                    "Selected Java path:\n{}",
                    path.as_ref().unwrap_or(&"Path not selected".to_string())
                ),
                Lang::Russian => format!(
                    "Выбранный путь к Java:\n{}",
                    path.as_ref().unwrap_or(&"Путь не выбран".to_string())
                ),
            },
            LangMessage::JavaXMX => match lang {
                Lang::English => "Java Xmx".to_string(),
                Lang::Russian => "Java Xmx".to_string(),
            },
            LangMessage::SelectJavaPath => match lang {
                Lang::English => "Select Java path".to_string(),
                Lang::Russian => "Выберите путь к Java".to_string(),
            },
            LangMessage::Launch => match lang {
                Lang::English => "Launch".to_string(),
                Lang::Russian => "Запустить".to_string(),
            },
            LangMessage::LaunchError(e) => match lang {
                Lang::English => format!("Error launching: {}", e),
                Lang::Russian => format!("Ошибка запуска: {}", e),
            },
            LangMessage::Running => match lang {
                Lang::English => "Running...".to_string(),
                Lang::Russian => "Запущено...".to_string(),
            },
            LangMessage::Language => match lang {
                Lang::English => "Language".to_string(),
                Lang::Russian => "Язык".to_string(),
            },
            LangMessage::LanguageName => match lang {
                Lang::English => "English".to_string(),
                Lang::Russian => "Русский".to_string(),
            },
            LangMessage::DownloadingUpdate => match lang {
                Lang::English => "Downloading update...".to_string(),
                Lang::Russian => "Загрузка обновления...".to_string(),
            },
            LangMessage::CheckingForUpdates => match lang {
                Lang::English => "Checking for updates...".to_string(),
                Lang::Russian => "Проверка обновлений...".to_string(),
            },
            LangMessage::Launching => match lang {
                Lang::English => "Launching...".to_string(),
                Lang::Russian => "Запуск...".to_string(),
            },
            LangMessage::ErrorCheckingForUpdates(e) => match lang {
                Lang::English => format!("Error checking for updates: {}", e),
                Lang::Russian => format!("Ошибка проверки обновлений: {}", e),
            },
            LangMessage::ErrorDownloadingUpdate(e) => match lang {
                Lang::English => format!("Error downloading update: {}", e),
                Lang::Russian => format!("Ошибка загрузки обновления: {}", e),
            },
            LangMessage::ErrorReadOnly => match lang {
                Lang::English => {
                    if cfg!(target_os = "macos") {
                        "Error: read-only mode. If running from a disk image, copy to Applications"
                            .to_string()
                    } else {
                        "Error: read-only mode".to_string()
                    }
                }
                Lang::Russian => {
                    if cfg!(target_os = "macos") {
                        "Ошибка: режим только для чтения. Если лаунчер запущен из образа диска, скопируйте в Applications".to_string()
                    } else {
                        "Ошибка: режим только для чтения".to_string()
                    }
                }
            },
            LangMessage::ProceedToLauncher => match lang {
                Lang::English => "Proceed to launcher".to_string(),
                Lang::Russian => "Перейти к лаунчеру".to_string(),
            },
            LangMessage::Authorization => match lang {
                Lang::English => "Authorization".to_string(),
                Lang::Russian => "Авторизация".to_string(),
            },
            LangMessage::Modpacks => match lang {
                Lang::English => "Modpacks".to_string(),
                Lang::Russian => "Модпаки".to_string(),
            },
            LangMessage::ForceOverwrite => match lang {
                Lang::English => "Overwrite optional files".to_string(),
                Lang::Russian => "Перезаписать необязательные файлы".to_string(),
            },
            LangMessage::ForceOverwriteWarning => match lang {
                Lang::English => "Warning: this may overwrite such files as configs, server list, etc.".to_string(),
                Lang::Russian => "Внимание: это может перезаписать такие файлы как настройки, список серверов и т.д.".to_string(),
            },
            LangMessage::OpenLauncherDirectory => match lang {
                Lang::English => "Open launcher directory".to_string(),
                Lang::Russian => "Открыть папку лаунчера".to_string(),
            },
            LangMessage::KillMinecraft => match lang {
                Lang::English => "Kill Minecraft".to_string(),
                Lang::Russian => "Закрыть Minecraft".to_string(),
            },
            LangMessage::CloseLauncherAfterLaunch => match lang {
                Lang::English => "Close launcher after launch".to_string(),
                Lang::Russian => "Закрыть лаунчер после запуска".to_string(),
            },
            LangMessage::DownloadAndLaunch => match lang {
                Lang::English => "Download and launch".to_string(),
                Lang::Russian => "Загрузить и запустить".to_string(),
            },
            LangMessage::CancelLaunch => match lang {
                Lang::English => "Cancel launch".to_string(),
                Lang::Russian => "Отменить запуск".to_string(),
            },
        }
    }
}
