use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum Lang {
    English,
    Russian,
}

pub struct Localization {
    pub exiting: &'static str,
    pub error_during_auth: &'static str,
    pub error_during_user_info: &'static str,
    pub error_use_internet_for_first_connection: &'static str,
    pub checking_files: &'static str,
    pub downloading_files: &'static str,
    pub no_remote_modpacks: &'static str,
    pub no_local_modpacks: &'static str,
    pub select_modpack: &'static str,
    pub select_menu_help: &'static str,
}

const RUSSIAN_LOC: Localization = Localization {
    exiting: "Выход...",
    error_during_auth: "Ошибка при аутентификации",
    error_during_user_info: "Ошибка при получении информации о пользователе",
    error_use_internet_for_first_connection: "Используйте интернет для первого подключения",
    checking_files: "Проверка файлов...",
    downloading_files: "Загрузка файлов...",
    no_remote_modpacks: "Список модпаков на сервере пуст",
    no_local_modpacks: "Запуск без загруженных модпаков должен быть с интернетом",
    select_modpack: "Выберите модпак",
    select_menu_help: "↑↓ для перемещения, enter для выбора, ввод для поиска",
};

const ENGLISH_LOC: Localization = Localization {
    exiting: "Exiting...",
    error_during_auth: "Error during authentication",
    error_during_user_info: "Error during user info retrieval",
    error_use_internet_for_first_connection: "Use internet for first connection",
    checking_files: "Checking files...",
    downloading_files: "Downloading files...",
    no_remote_modpacks: "No modpacks on server",
    no_local_modpacks: "Running without downloaded modpacks should be with internet connection",
    select_modpack: "Select modpack",
    select_menu_help: "", // unused, using default
};  

pub fn get_loc(lang: &Lang) -> Localization {
    match lang {
        Lang::English => ENGLISH_LOC,
        Lang::Russian => RUSSIAN_LOC,
    }
}
