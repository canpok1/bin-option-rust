use common_lib::{error::MyResult, mysql::{self, client::{DefaultClient, Client}}};
use log::{error, info};
use smartcore::{linalg::{naive::dense_matrix::DenseMatrix}, model_selection::{train_test_split}, ensemble::random_forest_regressor::RandomForestRegressor, metrics::{mean_squared_error}};

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

    info!("start training");
    match training(&mysql_cli) {
        Ok(_) => {
            info!("finished training");
        }
        Err(err) => {
            error!("failed to training, error:{}", err);
        }
    }
}

fn training(mysql_cli: &DefaultClient) -> MyResult<()> {
    let test_data_size = 50;
    let test_data_count = 500;
    let mut x:Vec<Vec<f64>> = vec![];
    let mut y:Vec<f64> = vec![];

    mysql_cli.with_transaction(|tx| -> MyResult<()> {
        let rates = mysql_cli.select_rates_for_training( tx, "USDJPY", None, None)?;
        for offset in 0..test_data_count {
            let mut rate:Vec<f64> = vec![];
            for index in 0..test_data_size {
                rate.push(rates[index + offset].rate.clone());
            }
            x.push(rate);
            y.push(rates[test_data_size + offset + 5].rate);
        }
        Ok(())
    })?;
    let matrix = DenseMatrix::from_2d_vec(&x);

    let mut best_model:Option<RandomForestRegressor<_>> = None;
    let mut best_mne:Option<f64> = None;
    for _ in 1..10 {
        let (x_train, x_test, y_train, y_test) = train_test_split(&matrix, &y, 0.2, true);

        let model = RandomForestRegressor::fit(&x_train, &y_train, Default::default())?;
        let y_hat = model.predict(&x_test)?;
        let mne = mean_squared_error(&y_test, &y_hat);
        info!("MSE Random Forest: {}", mne);

        if best_mne.is_none() || best_mne.unwrap() > mne {
            best_model = Some(model);
            best_mne = Some(mne);
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
        info!("[no{}] want: {}, got: {}", row+1, y_test[row], y_hat[row]);
    }

    Ok(())
}
