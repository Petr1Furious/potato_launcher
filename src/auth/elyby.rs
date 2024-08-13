use super::base::{AuthProvider, UserInfo};

use reqwest::Error;

pub const ELY_BY_BASE : &str = "https://ely.by/";

pub struct ElyByAuthProvider {
}

impl AuthProvider for ElyByAuthProvider {
    async fn authenticate(&mut self) -> Result<String, Error> {
        unimplemented!()
    }

    async fn get_user_info(&self, _token: &String) -> Result<UserInfo, Error> {
        unimplemented!()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
