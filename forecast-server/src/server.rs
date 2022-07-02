use chrono::{Utc, Duration};
use forecast_server_lib::{
    models::{self, RatesPost201Response},
    server::MakeService,
    Api,
    ForecastAfter5minRateIdGetResponse,
    RatesPostResponse,
};
use async_trait::async_trait;
use common_lib::{
    mysql::{self, client::Client},
    domain::model::RateForForecast,
};
use log::info;
use swagger::{auth::MakeAllowAllAuthenticator, ApiError, EmptyContext, Has, XSpanIdString};

pub async fn run(addr: &str, mysql_cli: mysql::client::DefaultClient) {
    let addr = addr.parse().expect("Failed to parse bind address");

    let server = Server::new(mysql_cli);

    let service = MakeService::new(server);

    let service = MakeAllowAllAuthenticator::new(service, "cosmo");

    let service =
        forecast_server_lib::server::context::MakeAddContext::<_, EmptyContext>::new(service);

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
    /// 5分後の予想を取得します
    async fn forecast_after5min_rate_id_get(
        &self,
        rate_id: String,
        context: &C) -> Result<ForecastAfter5minRateIdGetResponse, ApiError>
    {
        let context = context.clone();
        info!("forecast_after5min_rate_id_get(\"{}\") - X-Span-ID: {:?}", rate_id, context.get().0.clone());
        Err(ApiError("Generic failure".into()))
    }

    /// レート履歴を新規登録します
    async fn rates_post(
        &self,
        history: models::History,
        context: &C) -> Result<RatesPostResponse, ApiError>
    {
        let context = context.clone();
        info!("rates_post({:?}) - X-Span-ID: {:?}", history, context.get().0.clone());

        if history.rate_histories.is_empty() {
            return Ok(RatesPostResponse::Status400(models::Error {
                message: "parameter is invalid, rate_histories is empty.".to_string(),
            }));
        }

        let expire = (Utc::now() + Duration::hours(12)).naive_utc();
        let mut id:Option<String> = None;
        match self.mysql_cli.with_transaction(
            |tx| {
                let rate = RateForForecast::new(
                    history.pair.clone(),
                    history.rate_histories.clone(),
                    expire.clone(),
                    "inserted by forecast-server".to_string(),
                )?;

                id = Some(self.mysql_cli.insert_rates_for_forecast(tx, &rate)?);
                Ok(())
            }
        ) {
            Ok(_) => {
                Ok(RatesPostResponse::Status201(RatesPost201Response{
                    rate_id: id.unwrap(), expire: expire.format("%Y-%m-%d %H:%M:%S").to_string(),
                }))
            }
            Err(err) => {
                Ok(RatesPostResponse::Status500(models::Error {
                    message: format!("internal server error, {}", err),
                }))
            }
        }
    }
}
