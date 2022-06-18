use log::info;

mod server;

fn init_logger() {
    env_logger::init();
}

#[tokio::main]
async fn main() {
    init_logger();

    let addr = "127.0.0.1:18080";
    info!("start RateGateway {}", addr);
    server::run(addr).await;
}
