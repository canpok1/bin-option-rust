extern crate common_lib;

use common_lib::{
    batch,
    domain::model::ForecastResult,
    error::MyResult,
    mysql::{
        self,
        client::{Client, DefaultClient},
    },
};
use log::{error, info, warn};

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

    let mysql_cli: DefaultClient;
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
        info!("start forecast");
        match run(&config, &mysql_cli) {
            Ok(_) => {
                info!("finished forecast");
            }
            Err(err) => {
                error!("failed to forecast, error:{}", err);
            }
        }
    }) {
        error!("failed to start scheduler, error: {}", err);
    }
}

fn run(config: &config::Config, mysql_cli: &DefaultClient) -> MyResult<()> {
    mysql_cli.with_transaction(|tx| -> MyResult<()> {
        let models = mysql_cli.select_forecast_models(tx, &config.currency_pair)?;
        let rates = mysql_cli.select_rates_for_forecast_unforecasted(tx, &config.currency_pair)?;
        info!(
            "model count: {}, rates count: {}",
            models.len(),
            rates.len()
        );

        let mut results: Vec<ForecastResult> = vec![];
        for rate in &rates {
            let rate_size = rate.histories.len();
            if rate_size != config.forecast_input_size {
                warn!(
                    "rate size is unsupported size. rate_id: {}, size: {}, supported: {}, ",
                    rate.id, rate_size, config.forecast_input_size
                );
                continue;
            }

            for model in &models {
                let result = ForecastResult::new(
                    rate.id.to_string(),
                    model.get_no()?,
                    0,
                    model.predict(&rate.histories)?,
                    "after5min".to_string(),
                )?;
                info!(
                    "pair: {}, model_no: {}, rate_id: {}, result: {}",
                    model.get_pair()?,
                    result.model_no,
                    result.rate_id,
                    result.result
                );

                results.push(result);
            }
        }

        mysql_cli.insert_forecast_results(tx, &results)?;

        Ok(())
    })
}
