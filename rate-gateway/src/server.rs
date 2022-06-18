use rate_gateway_lib::{
    models::{self, PostSuccess},
    server::MakeService,
    Api, RatesPairPostResponse,
};

use async_trait::async_trait;
use common_lib::mysql::{self, client::Client};
use log::info;
use swagger::{auth::MakeAllowAllAuthenticator, ApiError, EmptyContext, Has, XSpanIdString};

pub async fn run(addr: &str, mysql_cli: mysql::client::DefaultClient) {
    let addr = addr.parse().expect("Failed to parse bind address");

    let server = Server::new(mysql_cli);

    let service = MakeService::new(server);

    let service = MakeAllowAllAuthenticator::new(service, "cosmo");

    let service =
        rate_gateway_lib::server::context::MakeAddContext::<_, EmptyContext>::new(service);

    hyper::server::Server::bind(&addr)
        .serve(service)
        .await
        .unwrap()
}

#[derive(Clone)]
pub struct Server {
    mysql_cli: mysql::client::DefaultClient,
}

impl Server {
    pub fn new(mysql_cli: mysql::client::DefaultClient) -> Self {
        Server {
            mysql_cli: mysql_cli,
        }
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
        rates: &Vec<models::Rate>,
        context: &C,
    ) -> Result<RatesPairPostResponse, ApiError> {
        let context = context.clone();
        info!(
            "rates_pair_post(\"{}\", {:?}) - X-Span-ID: {:?}",
            pair,
            rates,
            context.get().0.clone()
        );

        let rates = rates
            .iter()
            .map(|rate| mysql::model::RateForTraining::new(&pair, &rate.time, rate.value))
            .collect();
        if let Err(err) = rates {
            return Ok(RatesPairPostResponse::Status400(models::Error {
                message: format!("parameter is invalid, {}", err),
            }));
        }
        let rates = rates.unwrap();

        if let Err(err) = self.mysql_cli.insert_rates_for_training(&rates) {
            return Ok(RatesPairPostResponse::Status500(models::Error {
                message: format!("internal server error, {}", err),
            }));
        }

        Ok(RatesPairPostResponse::Status201(PostSuccess {
            count: rates.len() as i64,
        }))
    }
}
