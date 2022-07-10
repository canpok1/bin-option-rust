use chrono::{Duration, NaiveDateTime, Utc};
use common_lib::{
    batch,
    domain::{
        model::{FeatureParams, TrainingDataset},
        service::Converter,
    },
    error::MyResult,
    mysql::{
        self,
        client::{Client, DefaultClient},
    },
};
use log::{debug, error, info, warn};

use crate::training::Trainer;

mod config;
mod training;
mod util;

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
        info!("start training");
        match training(&config, &mysql_cli) {
            Ok(_) => {
                info!("finished training");
            }
            Err(err) => {
                error!("failed to training, error:{}", err);
            }
        }
    }) {
        error!("failed to start scheduler, error: {}", err);
    }
}

fn training(config: &config::Config, mysql_cli: &DefaultClient) -> MyResult<()> {
    let end = Utc::now().naive_utc();
    let begin = (Utc::now() - Duration::hours(config.training_data_range_hour)).naive_utc();

    let p = FeatureParams::new_default();

    let (org_x, org_y) = load_data(config, mysql_cli, begin, end, &p)?;
    if org_x.len() < config.training_data_required_count {
        warn!(
            "training data is too little. skip training. count:{}",
            org_x.len()
        );
        return Ok(());
    }
    debug!(
        "loaded data count is (org_x,org_y)=({},{})",
        org_x.len(),
        org_y.len()
    );

    let (train_base_x, test_x, train_base_y, test_y) = util::train_test_split(&org_x, &org_y, 0.2)?;
    debug!(
        "training base data count is (x,y)=({},{})",
        train_base_x.len(),
        train_base_y.len()
    );
    debug!(
        "test data count is (x,y)=({},{})",
        test_x.len(),
        test_y.len()
    );

    let trainer = Trainer { config, mysql_cli };
    trainer.training(&p, &train_base_x, &train_base_y, &test_x, &test_y)?;

    Ok(())
}

fn load_data(
    config: &config::Config,
    mysql_cli: &DefaultClient,
    begin: NaiveDateTime,
    end: NaiveDateTime,
    params: &FeatureParams,
) -> MyResult<(Vec<Vec<f64>>, Vec<f64>)> {
    let mut x: Vec<Vec<f64>> = vec![];
    let mut y: Vec<f64> = vec![];

    let converter = Converter {};

    mysql_cli.with_transaction(|tx| -> MyResult<()> {
        debug!("fetch rates. begin:{}, end:{}", begin, end);

        let rates = mysql_cli.select_rates_for_training(
            tx,
            &config.currency_pair,
            Some(begin),
            Some(end),
        )?;
        debug!("fetched rates count: {}", rates.len());

        for offset in 0..rates.len() {
            let truth =
                rates.get(offset + config.forecast_input_size - 1 + config.forecast_offset_minutes);
            if truth.is_none() {
                break;
            }

            let mut before: f64 = 0.0;
            let mut same_count = 0;
            let mut data: Vec<f64> = vec![];
            for index in offset..offset + config.forecast_input_size {
                data.push(rates[index].rate.clone());
                if rates[index].rate == before {
                    same_count += 1;
                }
                before = rates[index].rate.clone();
            }
            if same_count > (data.len() / 2) {
                continue;
            }
            // データ数を偶数にしないとLinearの学習でエラーになるようなので偶数になるよう調整
            if offset == rates.len() && x.len() % 2 == 0 {
                continue;
            }
            x.push(converter.convert_to_features(&data, params)?);
            y.push(truth.unwrap().rate);
        }

        Ok(())
    })?;
    Ok((x, y))
}

fn save_training_datasets(
    config: &config::Config,
    mysql_cli: &DefaultClient,
    x: &Vec<Vec<f64>>,
    y: &Vec<f64>,
) -> MyResult<()> {
    let mut datasets: Vec<TrainingDataset> = vec![];
    for i in 0..x.len() {
        let dataset = TrainingDataset::new(
            config.currency_pair.clone(),
            x[i].clone(),
            y[i].clone(),
            "inserted by training-batch".to_string(),
        )?;
        datasets.push(dataset);
    }
    mysql_cli.with_transaction(|tx| mysql_cli.insert_training_datasets(tx, &datasets))?;

    Ok(())
}
