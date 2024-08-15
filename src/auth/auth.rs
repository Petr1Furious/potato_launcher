use super::base;
use super::base::AuthProvider;

use crate::lang;
use crate::config::runtime_config;
use crate::utils::print_error_and_exit;

pub async fn auth_and_save(config: &mut runtime_config::Config) -> bool {
    let mut online = true;

    let mut tried_auth = false;
    for _ in 0..2 {
        let mut auth_provider = base::get_auth_provider(config.lang.clone());
        let token: String;
        if config.token.is_some() {
            token = config.token.clone().unwrap();
        } else {
            if tried_auth {
                print_error_and_exit(lang::get_loc(&config.lang).error_during_auth);
            }
            tried_auth = true;

            match auth_provider.authenticate().await {
                Ok(t) => token = t,
                Err(e) => {
                    let e = e.downcast_ref::<reqwest::Error>();
                    if e.is_some() && e.unwrap().is_connect() {
                        online = false;
                        break;
                    } else {
                        print_error_and_exit(lang::get_loc(&config.lang).error_during_auth);
                    }
                }
            }
            config.token = Some(token.clone());
        }
    
        let user = auth_provider.get_user_info(&token).await;
        match user {
            Ok(user) => {
                config.user_info = Some(user);
                break;
            }
            Err(e) => {
                let e = e.downcast_ref::<reqwest::Error>();
                if let Some(e) = e {
                    if e.is_connect() {
                        online = false;
                        break;
                    } else if let Some(status) = e.status() {
                        if status.is_client_error() {
                            config.token = None;
                            continue;
                        }
                    }
                }
                print_error_and_exit(format!("{}: {:?}", lang::get_loc(&config.lang).error_during_user_info, e).as_str());
            }
        }
    }

    if !online {
        if config.user_info.is_none() {
            print_error_and_exit(lang::get_loc(&config.lang).error_use_internet_for_first_connection);
        }
    }

    runtime_config::save_config(&config);

    return online;
}
