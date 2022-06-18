use log::{error, info};

mod config;
mod server;

fn init_logger() {
    env_logger::init();
}

#[tokio::main]
async fn main() {
    init_logger();

    let config = envy::from_env::<config::Config>();
    if let Err(error) = config {
        error!("failed to load config, error: {}", error);
        return;
    }

    let addr = config.unwrap().get_address();
    info!("start RateGateway {}", addr);
    server::run(&addr).await;
}
