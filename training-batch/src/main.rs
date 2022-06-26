use chrono::{Duration, Utc, NaiveDateTime};
use common_lib::{
    error::MyResult,
    mysql::{
        self,
        client::{DefaultClient, Client}, model::ForecastModel,
    },
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
    let end = Utc::now().naive_utc();
    let begin = (Utc::now() - Duration::hours(config.training_data_range_hour)).naive_utc();

    let (org_x, org_y) = load_data(config, mysql_cli, begin, end)?;
    if org_x.len() < config.training_data_required_count {
        warn!("training data is too little. skip training. count:{}", org_x.len());
        return Ok(());
    }
    let matrix = DenseMatrix::from_2d_vec(&org_x);
    let (train_base_x, test_x, train_base_y, test_y) = train_test_split(&matrix, &org_y, 0.2, true);

    let mut best_model:Option<RandomForestRegressor<_>> = None;
    let mut best_mne:Option<f64> = None;

    let existing_model = load_existing_model(config, mysql_cli)?;
    if let Some(m) = existing_model {
        let model = bincode::deserialize::<RandomForestRegressor<f64>>(&m.data)?;

        let y = model.predict(&test_x)?;
        let mne = mean_squared_error(&test_y, &y);
        info!("MSE Random Forest: {}", mne);

        best_model = Some(model);
        best_mne = Some(mne);
    }

    for _ in 1..config.training_count {
        let (train_x, _, train_y, _) = train_test_split(&train_base_x, &train_base_y, 0.2, true);

        let model = RandomForestRegressor::fit(&train_x, &train_y, Default::default())?;
        let y = model.predict(&test_x)?;
        let mne = mean_squared_error(&test_y, &y);
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

    let best_model = best_model.unwrap();
    let y = best_model.predict(&test_x)?;
    for row in 0..test_y.len() {
        let want = test_y[row];
        let got = y[row];
        let diff = want - got;
        info!("[no{:02}] want: {:.4}, got: {:.4}, diff: {:.4}", row+1, want, got, diff);
    }

    save_model(config, mysql_cli, &best_model)?;

    Ok(())
}

fn load_data(config: &config::Config, mysql_cli: &DefaultClient, begin: NaiveDateTime, end: NaiveDateTime) -> MyResult<(Vec<Vec<f64>>, Vec<f64>)> {
    let mut x:Vec<Vec<f64>> = vec![];
    let mut y:Vec<f64> = vec![];

    mysql_cli.with_transaction(|tx| -> MyResult<()> {
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

        Ok(())
    })?;
    Ok((x, y))
}

fn load_existing_model(config: &config::Config, mysql_cli: &DefaultClient) -> MyResult<Option<ForecastModel>> {
    let mut model:Option<ForecastModel> = None;
    mysql_cli.with_transaction(|tx| -> MyResult<()> {
        model = mysql_cli.select_forecast_model(tx, &config.currency_pair, 0)?;
        Ok(())
    })?;
    Ok(model)
}


fn save_model(config: &config::Config, mysql_cli: &DefaultClient, model: &RandomForestRegressor<f64>) -> MyResult<()> {
    mysql_cli.with_transaction(|tx| {
        let bin = bincode::serialize(model)?;
        let m = ForecastModel::new(config.currency_pair.clone(), config.forecast_model_no, bin, "test".to_string())?;
        mysql_cli.upsert_forecast_model(tx, &m)?;
        Ok(())
    })?;
    Ok(())
}