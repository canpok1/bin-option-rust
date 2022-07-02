extern crate common_lib;
extern crate rate_gateway_lib;

use common_lib::mysql;
use log::{error, info};

mod config;
mod server;

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
    match mysql::util::make_cli() {
        Ok(cli) => {
            mysql_cli = cli;
        }
        Err(err) => {
            error!("failed to make mysql client, error: {}", err);
            return;
        }
    }

    let addr = config.get_address();
    info!("start RateGateway {}", addr);
    server::run(&addr, mysql_cli).await;
}
