// use crate::config::runtime_config::Config;
// use crate::lang;
// use inquire::Select;

// use crate::modpack::index::ModpackIndex;

// fn get_select_with_language<T>(config: &Config, items: Vec<T>) -> Select<T>
// where
//     T: std::fmt::Display,
// {
//     let mut select = Select::new(lang::get_loc(&config.lang).select_modpack, items);
//     if config.lang != lang::Lang::English {
//         select = select.with_help_message(lang::get_loc(&config.lang).select_menu_help);
//     }
//     select
// }

// pub fn select_modpack(config: &Config, modpacks: &Vec<ModpackIndex>) -> String {
//     clearscreen::clear().unwrap();
//     let items: Vec<String> = modpacks
//         .iter()
//         .map(|x| x.modpack_name.clone())
//         .collect();
//     get_select_with_language(config, items).prompt().unwrap()
// }
