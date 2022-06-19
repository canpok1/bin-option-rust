extern crate common_lib;

use chrono::{Utc, Duration};
use common_lib::{mysql::{self, client::Client}, error::MyResult};
use log::{error, info};

mod config;

fn init_logger() {
    env_logger::init();
}


#[tokio::main]
async fn main() {
    init_logger();

    let config: config::Config;
    match envy::from_env::<config::Config>() {
        Ok(c) => {
            config = c;
        }
        Err(err) => {
            error!("failed to load config, error: {}", err);
            return;
        }
    }

    let mysql_cli: mysql::client::DefaultClient;
    match mysql::client::DefaultClient::new(
        &config.db_user_name,
        &config.db_password,
        &config.db_host,
        config.db_port,
        &config.db_name,
    ) {
        Ok(cli) => {
            mysql_cli = cli;
        }
        Err(err) => {
            error!("failed to load config, error: {}", err);
            return;
        }
    }

    info!("start DataCleanBatch, expire_date:{}", config.expire_date_count);

    let border = (Utc::now() - Duration::days(config.expire_date_count)).naive_utc();
    match mysql_cli.with_transaction(|tx| -> MyResult<()> {
        mysql_cli.delete_old_rates_for_training(tx, &border)
    }) {
        Ok(_) => {
            info!("successful cleaning table 'rate_for_training', border:{}", border);
        }
        Err(err) => {
            error!("failed to clean table 'rate_for_training', border:{}, error: {}", border, err);
        }
    };
}
