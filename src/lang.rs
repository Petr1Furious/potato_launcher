use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum Lang {
    English,
    Russian,
}

#[derive(Clone, PartialEq, Debug)]
pub enum LangMessage {
    AuthMessage{url: String},
    NoConnectionToAuthServer,
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
    DownloadJava{version: String},
    JavaInstalled{version: String},
    Launch,
    LaunchError(String),
    Running,
    Language,
    LanguageName,
}

impl LangMessage {
    pub fn to_string(&self, lang: &Lang) -> String {
        match self {
            LangMessage::AuthMessage { url: _ } => {
                match lang {
                    Lang::English => "Authorize in the browser window.\nOr open the link manually.".to_string(),
                    Lang::Russian => "Авторизуйтесь в открывшемся окне браузера.\nИли откройте ссылку вручную.".to_string(),
                }
            }
            LangMessage::NoConnectionToAuthServer => {
                match lang {
                    Lang::English => "No connection to the authorization server".to_string(),
                    Lang::Russian => "Нет подключения к серверу авторизации".to_string(),
                }
            }
            LangMessage::AuthError(e) => {
                match lang {
                    Lang::English => format!("Authorization error: {}", e),
                    Lang::Russian => format!("Ошибка авторизации: {}", e),
                }
            }
            LangMessage::AuthorizedAs(username) => {
                match lang {
                    Lang::English => format!("Authorized as {}", username),
                    Lang::Russian => format!("Авторизован как {}", username),
                }
            }
            LangMessage::Authorizing => {
                match lang {
                    Lang::English => "Authorizing...".to_string(),
                    Lang::Russian => "Авторизация...".to_string(),
                }
            }
            LangMessage::Authorize => {
                match lang {
                    Lang::English => "Authorize".to_string(),
                    Lang::Russian => "Авторизоваться".to_string(),
                }
            }
            LangMessage::FetchingModpackIndexes => {
                match lang {
                    Lang::English => "Fetching modpack list...".to_string(),
                    Lang::Russian => "Получение списка модпаков...".to_string(),
                }
            }
            LangMessage::FetchedRemoteIndexes => {
                match lang {
                    Lang::English => "Fetched remote modpack list".to_string(),
                    Lang::Russian => "Получен список модпаков с сервера".to_string(),
                }
            }
            LangMessage::NoConnectionToIndexServer => {
                match lang {
                    Lang::English => "No connection to the modpack server".to_string(),
                    Lang::Russian => "Нет подключения к серверу модпаков".to_string(),
                }
            }
            LangMessage::ErrorFetchingRemoteIndexes(s) => {
                match lang {
                    Lang::English => format!("Error fetching remote modpack list: {}", s),
                    Lang::Russian => format!("Ошибка получения списка модпаков с сервера: {}", s),
                }
            }
            LangMessage::FetchIndexes => {
                match lang {
                    Lang::English => "Fetch modpack list".to_string(),
                    Lang::Russian => "Получить список модпаков".to_string(),
                }
            }
            LangMessage::SelectModpack => {
                match lang {
                    Lang::English => "Select modpack".to_string(),
                    Lang::Russian => "Выберите модпак".to_string(),
                }
            }
            LangMessage::NoIndexes => {
                match lang {
                    Lang::English => "No modpacks fetched".to_string(),
                    Lang::Russian => "Список модпаков пуст".to_string(),
                }
            }
            LangMessage::CheckingFiles => {
                match lang {
                    Lang::English => "Checking files...".to_string(),
                    Lang::Russian => "Проверка файлов...".to_string(),
                }
            }
            LangMessage::DownloadingFiles => {
                match lang {
                    Lang::English => "Downloading files...".to_string(),
                    Lang::Russian => "Загрузка файлов...".to_string(),
                }
            }
            LangMessage::SyncModpack => {
                match lang {
                    Lang::English => "Sync modpack".to_string(),
                    Lang::Russian => "Синхронизировать модпак".to_string(),
                }
            }
            LangMessage::ModpackNotSynced => {
                match lang {
                    Lang::English => "Modpack not synced".to_string(),
                    Lang::Russian => "Модпак не синхронизирован".to_string(),
                }
            }
            LangMessage::SyncingModpack => {
                match lang {
                    Lang::English => "Syncing modpack...".to_string(),
                    Lang::Russian => "Синхронизация модпака...".to_string(),
                }
            }
            LangMessage::ModpackSynced => {
                match lang {
                    Lang::English => "Modpack up-to-date".to_string(),
                    Lang::Russian => "Модпак синхронизирован".to_string(),
                }
            }
            LangMessage::ModpackSyncError(e) => {
                match lang {
                    Lang::English => format!("Error syncing modpack: {}", e),
                    Lang::Russian => format!("Ошибка синхронизации модпака: {}", e),
                }
            }
            LangMessage::DownloadingJava => {
                match lang {
                    Lang::English => "Downloading Java...".to_string(),
                    Lang::Russian => "Загрузка Java...".to_string(),
                }
            }
            LangMessage::DownloadJava { version } => {
                match lang {
                    Lang::English => format!("Download Java {}", version),
                    Lang::Russian => format!("Загрузить Java {}", version),
                }
            }
            LangMessage::JavaInstalled { version } => {
                match lang {
                    Lang::English => format!("Java {} installed", version),
                    Lang::Russian => format!("Java {} установлена", version),
                }
            }
            LangMessage::Launch => {
                match lang {
                    Lang::English => "Launch".to_string(),
                    Lang::Russian => "Запустить".to_string(),
                }
            }
            LangMessage::LaunchError(e) => {
                match lang {
                    Lang::English => format!("Error launching: {}", e),
                    Lang::Russian => format!("Ошибка запуска: {}", e),
                }
            }
            LangMessage::Running => {
                match lang {
                    Lang::English => "Running...".to_string(),
                    Lang::Russian => "Запущено...".to_string(),
                }
            }
            LangMessage::Language => {
                match lang {
                    Lang::English => "Language".to_string(),
                    Lang::Russian => "Язык".to_string(),
                }
            }
            LangMessage::LanguageName => {
                match lang {
                    Lang::English => "English".to_string(),
                    Lang::Russian => "Русский".to_string(),
                }
            }
        }
    }
}
