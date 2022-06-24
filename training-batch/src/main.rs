use chrono::{Duration, Utc};
use common_lib::{
    error::MyResult,
    mysql::{
        self,
        client::{DefaultClient, Client},
    },
    domain::model::ForecastModel
};
use job_scheduler::{JobScheduler, Job};
use log::{error, info, warn, debug};
use smartcore::{
    linalg::naive::dense_matrix::DenseMatrix,
    model_selection::train_test_split,
    ensemble::random_forest_regressor::*,
    metrics::mean_squared_error
};

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

fn start_scheduler(config: &config::Config, mysql_cli: &mysql::client::DefaultClient) -> MyResult<()> {
    let mut sched = JobScheduler::new();

    info!("set cron schedule: {}", &config.cron_schedule);
    sched.add(Job::new(config.cron_schedule.parse()?, || {
        info!("start training");
        match training(config, mysql_cli) {
            Ok(_) => {
                info!("finished training");
            }
            Err(err) => {
                error!("failed to training, error:{}", err);
            }
        }
    }));

    loop {
        sched.tick();
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

fn training(config: &config::Config, mysql_cli: &DefaultClient) -> MyResult<()> {
    let mut x:Vec<Vec<f64>> = vec![];
    let mut y:Vec<f64> = vec![];
    let mut existing_model:Option<ForecastModel> = None;

    mysql_cli.with_transaction(|tx| -> MyResult<()> {
        let end = Utc::now().naive_utc();
        let begin = (Utc::now() - Duration::hours(10)).naive_utc();

        debug!("fetch rates. begin:{}, end:{}", begin, end);

        let rates = mysql_cli.select_rates_for_training(tx, &config.currency_pair, Some(begin), Some(end))?;
        debug!("fetched rates count: {}", rates.len());

        for offset in 0..rates.len() {
            let truth = rates.get(offset + config.forecast_input_size - 1 + config.forecast_offset_minutes);
            if truth.is_none() {
                break;
            }
            y.push(truth.unwrap().rate);

            let mut data:Vec<f64> = vec![];
            for index in offset..offset+config.forecast_input_size {
                data.push(rates[index].rate.clone());
            }
            x.push(data);
        }

        existing_model = mysql_cli.select_forecast_model(tx, &config.currency_pair, 0)?;

        Ok(())
    })?;
    if x.len() < config.training_data_required_count {
        warn!("training data is too little. skip training. count:{}", x.len());
        return Ok(());
    }

    let matrix = DenseMatrix::from_2d_vec(&x);

    let mut best_model:Option<RandomForestRegressor<_>> = None;
    let mut best_mne:Option<f64> = None;

    if let Some(m) = existing_model {
        let model = bincode::deserialize::<RandomForestRegressor<f64>>(&m.data)?;

        let (_x_train, x_test, _y_train, y_test) = train_test_split(&matrix, &y, 0.2, true);

        let y_hat = model.predict(&x_test)?;
        let mne = mean_squared_error(&y_test, &y_hat);
        info!("MSE Random Forest: {}", mne);

        best_model = Some(model);
        best_mne = Some(mne);
    }

    for _ in 1..config.training_count {
        let (x_train, x_test, y_train, y_test) = train_test_split(&matrix, &y, 0.2, true);

        let model = RandomForestRegressor::fit(&x_train, &y_train, Default::default())?;
        let y_hat = model.predict(&x_test)?;
        let mne = mean_squared_error(&y_test, &y_hat);
        info!("MSE Random Forest: {}", mne);

        if best_mne.is_none() || best_mne.unwrap() > mne {
            best_model = Some(model); best_mne = Some(mne);
        }
    }
    if let Some(mne) = best_mne {
        info!("Best MSE Random Forest: {}", mne);
    } else {
        error!("model not found");
        return Ok(());
    }

    let (_x_train, x_test, _y_train, y_test) = train_test_split(&matrix, &y, 0.2, true);
    let best_model = best_model.unwrap();
    let y_hat = best_model.predict(&x_test)?;
    for row in 0..y_test.len() {
        let want = y_test[row];
        let got = y_hat[row];
        let diff = want - got;
        info!("[no{:02}] want: {:.4}, got: {:.4}, diff: {:.4}", row+1, want, got, diff);
    }

    mysql_cli.with_transaction(|tx| {
        let bin = bincode::serialize(&best_model)?;
        let m = ForecastModel::new(config.currency_pair.clone(), config.forecast_model_no, bin, "test".to_string())?;
        mysql_cli.upsert_forecast_model(tx, &m)?;
        Ok(())
    })?;

    Ok(())
}
