use crate::error::MyResult;

use super::client::DefaultClient;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub db_host: String,
    pub db_port: u16,
    pub db_name: String,
    pub db_user_name: String,
    pub db_password: String,
}

pub fn make_cli() -> MyResult<DefaultClient> {
    let config: Config;
    match envy::from_env::<Config>() {
        Ok(c) => {
            config = c;
        }
        Err(err) => {
            return Err(Box::new(err));
        }
    }

    DefaultClient::new(
        &config.db_user_name,
        &config.db_password,
        &config.db_host,
        config.db_port,
        &config.db_name,
    )
}
