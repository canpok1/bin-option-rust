use rate_gateway_lib::{
    models::{self, PostSuccess},
    server::MakeService,
    Api, RatesPairPostResponse,
};

use async_trait::async_trait;
use log::info;
use swagger::{auth::MakeAllowAllAuthenticator, ApiError, EmptyContext, Has, XSpanIdString};

pub async fn run(addr: &str) {
    let addr = addr.parse().expect("Failed to parse bind address");

    let server = Server::new();

    let service = MakeService::new(server);

    let service = MakeAllowAllAuthenticator::new(service, "cosmo");

    let service =
        rate_gateway_lib::server::context::MakeAddContext::<_, EmptyContext>::new(service);

    hyper::server::Server::bind(&addr)
        .serve(service)
        .await
        .unwrap()
}

#[derive(Copy, Clone)]
pub struct Server {}

impl Server {
    pub fn new() -> Self {
        Server {}
    }
}

#[async_trait]
impl<C> Api<C> for Server
where
    C: Has<XSpanIdString> + Send + Sync,
{
    /// レートを新規登録します
    async fn rates_pair_post(
        &self,
        pair: String,
        rate: &Vec<models::Rate>,
        context: &C,
    ) -> Result<RatesPairPostResponse, ApiError> {
        let context = context.clone();
        info!(
            "rates_pair_post(\"{}\", {:?}) - X-Span-ID: {:?}",
            pair,
            rate,
            context.get().0.clone()
        );
        let response = RatesPairPostResponse::Status201(PostSuccess { count: 0 });
        Ok(response)
    }
}
