extern crate common_lib;

use chrono::{Utc, Duration};
use common_lib::{mysql::{self, client::Client}, error::MyResult};
use config::Config;
use job_scheduler::{Job, JobScheduler};
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

    if let Err(err) = start_scheduler(&config, &mysql_cli) {
        error!("failed to start scheduler, error: {}", err);
    }
}

fn start_scheduler(config: &Config, mysql_cli: &mysql::client::DefaultClient) -> MyResult<()> {
    let mut sched = JobScheduler::new();

    info!("set cron schedule: {}", &config.cron_schedule);
    sched.add(Job::new(config.cron_schedule.parse()?, || {
        run(&config, &mysql_cli);
    }));

    loop {
        sched.tick();
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

fn run(config: &Config, mysql_cli: &mysql::client::DefaultClient) {
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
