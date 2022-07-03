extern crate common_lib;

use chrono::{Duration, Utc};
use common_lib::{
    batch,
    error::MyResult,
    mysql::{self, client::Client},
};
use config::Config;
use log::{error, info};

mod config;

fn init_logger() {
    env_logger::init();
}

fn main() {
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
    match mysql::util::make_cli() {
        Ok(cli) => {
            mysql_cli = cli;
        }
        Err(err) => {
            error!("failed to make mysql client, error: {}", err);
            return;
        }
    }

    if let Err(err) = batch::util::start_scheduler(&config.cron_schedule, || {
        run(&config, &mysql_cli);
    }) {
        error!("failed to start scheduler, error: {}", err);
    }
}

fn run(config: &Config, mysql_cli: &mysql::client::DefaultClient) {
    info!(
        "start DataCleanBatch, expire_date:{}",
        config.expire_date_count
    );

    let border = (Utc::now() - Duration::days(config.expire_date_count)).naive_utc();
    match mysql_cli.with_transaction(|tx| -> MyResult<()> {
        mysql_cli.delete_old_rates_for_training(tx, &border)
    }) {
        Ok(_) => {
            info!(
                "successful cleaning table 'rate_for_training', border:{}",
                border
            );
        }
        Err(err) => {
            error!(
                "failed to clean table 'rate_for_training', border:{}, error: {}",
                border, err
            );
        }
    };
}
