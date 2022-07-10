extern crate common_lib;

use common_lib::{
    batch,
    domain::{
        model::{ForecastError, ForecastResult},
        service::convert_to_feature,
    },
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
        let mut errors: Vec<ForecastError> = vec![];
        for rate in &rates {
            let rate_size = rate.histories.len();
            for model in &models {
                let model_no = model.get_no()?;
                if let Some(e) = mysql_cli
                    .select_forecast_errors_by_rate_id_and_model_no(tx, &rate.id, model_no)?
                {
                    warn!(
                        "forecast skipped, error exists. id:{}, rate_id:{}, model_no:{}",
                        e.id, &rate.id, model_no
                    );
                    continue;
                }

                let input_data_size = model.get_input_data_size()?;
                if input_data_size != rate_size {
                    let record = ForecastError::new(
                        rate.id.clone(),
                        model.get_no()?,
                        "input data size is not supported".to_string(),
                        format!(
                            "size(model): {}, size(input data): {}",
                            input_data_size, rate_size
                        ),
                    )?;
                    warn!("forecast skipped, {}", record);
                    errors.push(record);

                    continue;
                }

                let features = convert_to_feature(&rate.histories, &model.get_feature_params()?)?;

                let result = ForecastResult::new(
                    rate.id.to_string(),
                    model.get_no()?,
                    0,
                    model.predict(&features)?,
                    "after5min".to_string(),
                )?;
                info!(
                    "forecast succeeded. pair: {}, model_no: {}, rate_id: {}, result: {}",
                    model.get_pair()?,
                    result.model_no,
                    result.rate_id,
                    result.result
                );

                results.push(result);
            }
        }

        mysql_cli.insert_forecast_results(tx, &results)?;
        mysql_cli.insert_forecast_errors(tx, &errors)?;

        Ok(())
    })
}
