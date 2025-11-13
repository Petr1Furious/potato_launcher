use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum Lang {
    English,
    Russian,
}

#[derive(Clone, PartialEq, Debug)]
pub enum LangMessage {
    AuthMessage {
        url: String,
    },
    DeviceAuthMessage {
        url: String,
        code: String,
    },
    AuthTimeout,
    UnknownAuthError,
    AuthorizeUsing(String),
    Authorizing,
    SelectInstance,
    NotSelected,
    NoInstances,
    CheckingFiles,
    DownloadingFiles,
    SyncInstance,
    InstanceNotSynced,
    InstanceSynced,
    NoConnectionToSyncServer,
    InstanceSyncError,
    CheckingJava,
    DownloadingJava,
    JavaInstalled {
        version: String,
    },
    NeedJava {
        version: String,
    },
    UnknownErrorDownloadingJava,
    NoConnectionToJavaServer,
    UnknownJavaVersion,
    Settings,
    InstanceSettings,
    SelectedJavaPath,
    NoJavaPath,
    AllocatedMemory,
    SelectJavaPath,
    #[cfg(target_os = "linux")]
    UseNativeGlfw,
    Launch,
    LaunchError,
    ProcessErrorCode(String),
    Running,
    LanguageName,
    DownloadingUpdate,
    CheckingForUpdates,
    Launching,
    ErrorCheckingForUpdates,
    ErrorDownloadingUpdate,
    NoConnectionToUpdateServer,
    ErrorReadOnly,
    ProceedToLauncher,
    Authorization,
    ForceOverwrite,
    ForceOverwriteWarning,
    KillMinecraft,
    HideLauncherAfterLaunch,
    DownloadAndLaunch,
    CancelLaunch,
    CancelDownload,
    Retry,
    OpenLogs,
    LoadingMetadata,
    MetadataErrorOffline,
    MetadataFetchError,
    NewInstance,
    NewInstanceName,
    GameVersion,
    Loader,
    LoaderVersion,
    InstanceNameExists,
    CreateInstance,
    CreatingInstance,
    Cancel,
    InstanceGenerateErrorOffline,
    InstanceGenerateError,
    LongTimeWarning,
    DeleteInstance,
    SelectInstanceToDelete,
    ConfirmDelete,
    Delete,
    AddAccount,
    SelectAccount,
    AddAndAuthenticate,
    Offline,
    FetchingRemote,
    ErrorFetchingRemote,
    InstanceSyncProgress,
    AddOfflineAccount,
    EnterNickname,
    GettingMetadata,
    NoMetadata,
    ReadLocalRemoteError,
    ReadLocalOffline,
    ErrorGettingMetadata,
    InvalidJavaInstallation,
    AddManifestUrl,
    ManifestSource,
    Default,
    CustomManifests,
    EnterManifestUrl,
    Add,
}

impl LangMessage {
    pub fn to_string(&self, lang: Lang) -> String {
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
            LangMessage::DeviceAuthMessage { url: _, code } => match lang {
                Lang::English => {
                    format!("Authorize in the browser window.\nOr open the link manually and enter the code: {code}")
                }
                Lang::Russian => {
                    format!("Авторизуйтесь в открывшемся окне браузера.\nИли откройте ссылку вручную и введите код: {code}")
                }
            },
            LangMessage::AuthTimeout => match lang {
                Lang::English => "Authorization timeout".to_string(),
                Lang::Russian => "Превышено время авторизации".to_string(),
            },
            LangMessage::UnknownAuthError => match lang {
                Lang::English => "Authorization error".to_string(),
                Lang::Russian => "Ошибка авторизации".to_string(),
            },
            LangMessage::AuthorizeUsing(app_name) => match lang {
                Lang::English => format!("Authorize using {app_name}"),
                Lang::Russian => format!("Авторизуйтесь через {app_name}"),
            },
            LangMessage::Authorizing => match lang {
                Lang::English => "Authorizing...".to_string(),
                Lang::Russian => "Авторизация...".to_string(),
            },
            LangMessage::SelectInstance => match lang {
                Lang::English => "Select instance:".to_string(),
                Lang::Russian => "Выберите версию:".to_string(),
            },
            LangMessage::NotSelected => match lang {
                Lang::English => "Not selected".to_string(),
                Lang::Russian => "Не выбрано".to_string(),
            },
            LangMessage::NoInstances => match lang {
                Lang::English => "No instances fetched".to_string(),
                Lang::Russian => "Список версий пуст".to_string(),
            },
            LangMessage::CheckingFiles => match lang {
                Lang::English => "Checking files...".to_string(),
                Lang::Russian => "Проверка файлов...".to_string(),
            },
            LangMessage::DownloadingFiles => match lang {
                Lang::English => "Downloading files...".to_string(),
                Lang::Russian => "Загрузка файлов...".to_string(),
            },
            LangMessage::SyncInstance => match lang {
                Lang::English => "Sync instance".to_string(),
                Lang::Russian => "Синхронизировать версию".to_string(),
            },
            LangMessage::InstanceNotSynced => match lang {
                Lang::English => "Instance not synced".to_string(),
                Lang::Russian => "Версия не синхронизирована".to_string(),
            },
            LangMessage::InstanceSynced => match lang {
                Lang::English => "Instance up-to-date".to_string(),
                Lang::Russian => "Версия синхронизирована".to_string(),
            },
            LangMessage::NoConnectionToSyncServer => match lang {
                Lang::English => "No connection to instance sync server".to_string(),
                Lang::Russian => "Нет подключения к серверу синхронизации версий".to_string(),
            },
            LangMessage::InstanceSyncError => match lang {
                Lang::English => "Error syncing instance".to_string(),
                Lang::Russian => "Ошибка синхронизации версии".to_string(),
            },
            LangMessage::CheckingJava => match lang {
                Lang::English => "Checking Java...".to_string(),
                Lang::Russian => "Проверка Java...".to_string(),
            },
            LangMessage::DownloadingJava => match lang {
                Lang::English => "Downloading Java...".to_string(),
                Lang::Russian => "Загрузка Java...".to_string(),
            },
            LangMessage::JavaInstalled { version } => match lang {
                Lang::English => format!("Java {version} installed"),
                Lang::Russian => format!("Java {version} установлена"),
            },
            LangMessage::NeedJava { version } => match lang {
                Lang::English => format!("Java {version} not installed"),
                Lang::Russian => format!("Java {version} не установлена"),
            },
            LangMessage::UnknownErrorDownloadingJava => match lang {
                Lang::English => "Error downloading Java".to_string(),
                Lang::Russian => "Ошибка загрузки Java".to_string(),
            },
            LangMessage::NoConnectionToJavaServer => match lang {
                Lang::English => "No connection to Java download server".to_string(),
                Lang::Russian => "Нет подключения к серверу загрузки Java".to_string(),
            },
            LangMessage::UnknownJavaVersion => match lang {
                Lang::English => "Unknown Java version".to_string(),
                Lang::Russian => "Неизвестная версия Java".to_string(),
            },
            LangMessage::Settings => match lang {
                Lang::English => "Settings".to_string(),
                Lang::Russian => "Настройки".to_string(),
            },
            LangMessage::InstanceSettings => match lang {
                Lang::English => "Instance settings".to_string(),
                Lang::Russian => "Настройки версии".to_string(),
            },
            LangMessage::SelectedJavaPath => match lang {
                Lang::English => "Selected Java path:".to_string(),
                Lang::Russian => "Выбранный путь к Java:".to_string(),
            },
            LangMessage::NoJavaPath => match lang {
                Lang::English => "No Java path selected".to_string(),
                Lang::Russian => "Путь к Java не выбран".to_string(),
            },
            LangMessage::AllocatedMemory => match lang {
                Lang::English => "Allocated memory".to_string(),
                Lang::Russian => "Выделенная память".to_string(),
            },
            LangMessage::SelectJavaPath => match lang {
                Lang::English => "Select Java path".to_string(),
                Lang::Russian => "Выберите путь к Java".to_string(),
            },
            #[cfg(target_os = "linux")]
            LangMessage::UseNativeGlfw => match lang {
                Lang::English => "Use native GLFW".to_string(),
                Lang::Russian => "Использовать нативную версию GLFW".to_string(),
            },
            LangMessage::Launch => match lang {
                Lang::English => "Launch".to_string(),
                Lang::Russian => "Запустить".to_string(),
            },
            LangMessage::LaunchError => match lang {
                Lang::English => "Error launching".to_string(),
                Lang::Russian => "Ошибка запуска".to_string(),
            },
            LangMessage::ProcessErrorCode(e) => match lang {
                Lang::English => format!("Process exited with code: {e}"),
                Lang::Russian => format!("Процесс завершился с кодом: {e}"),
            },
            LangMessage::Running => match lang {
                Lang::English => "Running...".to_string(),
                Lang::Russian => "Запущено...".to_string(),
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
            LangMessage::ErrorCheckingForUpdates => match lang {
                Lang::English => "Error checking for updates".to_string(),
                Lang::Russian => "Ошибка проверки обновлений".to_string(),
            },
            LangMessage::ErrorDownloadingUpdate => match lang {
                Lang::English => "Error downloading update".to_string(),
                Lang::Russian => "Ошибка загрузки обновления".to_string(),
            },
            LangMessage::NoConnectionToUpdateServer => match lang {
                Lang::English => "No connection to update server".to_string(),
                Lang::Russian => "Нет подключения к серверу обновлений".to_string(),
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
            LangMessage::ForceOverwrite => match lang {
                Lang::English => "Overwrite optional files".to_string(),
                Lang::Russian => "Перезаписать необязательные файлы".to_string(),
            },
            LangMessage::ForceOverwriteWarning => match lang {
                Lang::English => "Warning: this may overwrite such files as configs, server list, etc.".to_string(),
                Lang::Russian => "Внимание: это может перезаписать такие файлы как настройки, список серверов и т.д.".to_string(),
            },
            LangMessage::KillMinecraft => match lang {
                Lang::English => "Kill Minecraft".to_string(),
                Lang::Russian => "Закрыть Minecraft".to_string(),
            },
            LangMessage::HideLauncherAfterLaunch => match lang {
                Lang::English => "Hide launcher after launch".to_string(),
                Lang::Russian => "Скрыть лаунчер после запуска".to_string(),
            },
            LangMessage::DownloadAndLaunch => match lang {
                Lang::English => "Download and launch".to_string(),
                Lang::Russian => "Загрузить и запустить".to_string(),
            },
            LangMessage::CancelLaunch => match lang {
                Lang::English => "Cancel launch".to_string(),
                Lang::Russian => "Отменить запуск".to_string(),
            },
            LangMessage::CancelDownload => match lang {
                Lang::English => "Cancel download".to_string(),
                Lang::Russian => "Отменить загрузку".to_string(),
            },
            LangMessage::Retry => match lang {
                Lang::English => "Retry".to_string(),
                Lang::Russian => "Попробовать снова".to_string(),
            },
            LangMessage::OpenLogs => match lang {
                Lang::English => "Open logs folder".to_string(),
                Lang::Russian => "Открыть папку с логами".to_string(),
            },
            LangMessage::LoadingMetadata => match lang {
                Lang::English => "Loading metadata...".to_string(),
                Lang::Russian => "Загрузка метаданных...".to_string(),
            },
            LangMessage::MetadataErrorOffline => match lang {
                Lang::English => "No connection to metadata server".to_string(),
                Lang::Russian => "Нет подключения к серверу метаданных".to_string(),
            },
            LangMessage::MetadataFetchError => match lang {
                Lang::English => "Error fetching metadata".to_string(),
                Lang::Russian => "Ошибка получения метаданных".to_string(),
            },
            LangMessage::NewInstance => match lang {
                Lang::English => "New instance".to_string(),
                Lang::Russian => "Новая версия".to_string(),
            },
            LangMessage::NewInstanceName => match lang {
                Lang::English => "New instance name".to_string(),
                Lang::Russian => "Название новой версии".to_string(),
            },
            LangMessage::GameVersion => match lang {
                Lang::English => "Game version".to_string(),
                Lang::Russian => "Версия игры".to_string(),
            },
            LangMessage::Loader => match lang {
                Lang::English => "Loader".to_string(),
                Lang::Russian => "Лоадер".to_string(),
            },
            LangMessage::LoaderVersion => match lang {
                Lang::English => "Loader version".to_string(),
                Lang::Russian => "Версия лоадера".to_string(),
            },
            LangMessage::InstanceNameExists => match lang {
                Lang::English => "Instance name already exists".to_string(),
                Lang::Russian => "Версия с таким именем уже существует".to_string(),
            },
            LangMessage::CreateInstance => match lang {
                Lang::English => "Create instance".to_string(),
                Lang::Russian => "Создать версию".to_string(),
            },
            LangMessage::CreatingInstance => match lang {
                Lang::English => "Creating instance...".to_string(),
                Lang::Russian => "Создание версии...".to_string(),
            },
            LangMessage::Cancel => match lang {
                Lang::English => "Cancel".to_string(),
                Lang::Russian => "Отмена".to_string(),
            },
            LangMessage::InstanceGenerateErrorOffline => match lang {
                Lang::English => "Error generating instance: no connection".to_string(),
                Lang::Russian => "Ошибка создания версии: нет подключения".to_string(),
            },
            LangMessage::InstanceGenerateError => match lang {
                Lang::English => "Error generating instance".to_string(),
                Lang::Russian => "Ошибка создания версии".to_string(),
            },
            LangMessage::LongTimeWarning => match lang {
                Lang::English => "This may take a couple of minutes".to_string(),
                Lang::Russian => "Это может занять несколько минут".to_string(),
            },
            LangMessage::DeleteInstance => match lang {
                Lang::English => "Delete instance".to_string(),
                Lang::Russian => "Удалить версию".to_string(),
            },
            LangMessage::SelectInstanceToDelete => match lang {
                Lang::English => "Select instance to delete".to_string(),
                Lang::Russian => "Выберите версию для удаления".to_string(),
            },
            LangMessage::ConfirmDelete => match lang {
                Lang::English => "I understand that this action is irreversible".to_string(),
                Lang::Russian => "Я понимаю, что назад пути нет".to_string(),
            },
            LangMessage::Delete => match lang {
                Lang::English => "Delete".to_string(),
                Lang::Russian => "Удалить".to_string(),
            },
            LangMessage::AddAccount => match lang {
                Lang::English => "Add account".to_string(),
                Lang::Russian => "Добавить аккаунт".to_string(),
            },
            LangMessage::SelectAccount => match lang {
                Lang::English => "Select account".to_string(),
                Lang::Russian => "Выберите аккаунт".to_string(),
            },
            LangMessage::AddAndAuthenticate => match lang {
                Lang::English => "Add and authenticate".to_string(),
                Lang::Russian => "Добавить и авторизоваться".to_string(),
            },
            LangMessage::Offline => match lang {
                Lang::English => "Offline".to_string(),
                Lang::Russian => "Офлайн".to_string(),
            },
            LangMessage::FetchingRemote => match lang {
                Lang::English => "Fetching...".to_string(),
                Lang::Russian => "Загрузка...".to_string(),
            },
            LangMessage::ErrorFetchingRemote => match lang {
                Lang::English => "Error fetching".to_string(),
                Lang::Russian => "Ошибка загрузки".to_string(),
            },
            LangMessage::InstanceSyncProgress => match lang {
                Lang::English => "Instance sync progress".to_string(),
                Lang::Russian => "Прогресс синхронизации версии".to_string(),
            },
            LangMessage::AddOfflineAccount => match lang {
                Lang::English => "Add offline account".to_string(),
                Lang::Russian => "Добавить офлайн аккаунт".to_string(),
            },
            LangMessage::EnterNickname => match lang {
                Lang::English => "Enter nickname".to_string(),
                Lang::Russian => "Введите никнейм".to_string(),
            },
            LangMessage::GettingMetadata => match lang {
                Lang::English => "Getting metadata...".to_string(),
                Lang::Russian => "Получение метаданных...".to_string(),
            },
            LangMessage::NoMetadata => match lang {
                Lang::English => "No metadata".to_string(),
                Lang::Russian => "Метаданные отсутствуют".to_string(),
            },
            LangMessage::ReadLocalRemoteError => match lang {
                Lang::English => "Local metadata (fetch error)".to_string(),
                Lang::Russian => "Локальные метаданные (ошибка загрузки)".to_string(),
            },
            LangMessage::ReadLocalOffline => match lang {
                Lang::English => "Local metadata (offline)".to_string(),
                Lang::Russian => "Локальные метаданные (офлайн)".to_string(),
            },
            LangMessage::ErrorGettingMetadata => match lang {
                Lang::English => "Error getting metadata".to_string(),
                Lang::Russian => "Ошибка получения метаданных".to_string(),
            },
            LangMessage::InvalidJavaInstallation => match lang {
                Lang::English => "Invalid Java Installation".to_string(),
                Lang::Russian => "Неверная установка Java".to_string(),
            },
            LangMessage::AddManifestUrl => match lang {
                Lang::English => "➕ Add manifest URL".to_string(),
                Lang::Russian => "➕ Добавить URL-адрес манифеста".to_string(),
            },
            LangMessage::ManifestSource => match lang {
                Lang::English => "Manifest source".to_string(),
                Lang::Russian => "Источник манифеста".to_string(),
            },
            LangMessage::Default => match lang {
                Lang::English => "Default".to_string(),
                Lang::Russian => "По умолчанию".to_string(),
            },
            LangMessage::CustomManifests => match lang {
                Lang::English => "Custom manifests:".to_string(),
                Lang::Russian => "Custom manifests:".to_string(),
            },
            LangMessage::EnterManifestUrl => match lang {
                Lang::English => "Enter manifest URL".to_string(),
                Lang::Russian => "Введите URL-адрес манифеста".to_string(),
            },
            LangMessage::Add => match lang {
                Lang::English => "Add".to_string(),
                Lang::Russian => "Добавить".to_string(),
            },
        }
    }
}
