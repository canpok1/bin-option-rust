use chrono::{Duration, NaiveDateTime, Utc};
use common_lib::{
    batch,
    domain::model::{ForecastModel, TrainingDataset},
    error::MyResult,
    mysql::{
        self,
        client::{Client, DefaultClient},
    },
};
use log::{debug, error, info, warn};
use smartcore::{
    ensemble::random_forest_regressor::*,
    linalg::naive::dense_matrix::DenseMatrix,
    linear::{
        elastic_net::{ElasticNet, ElasticNetParameters},
        lasso::{Lasso, LassoParameters},
        linear_regression::LinearRegression,
        logistic_regression::LogisticRegression,
        ridge_regression::{RidgeRegression, RidgeRegressionParameters},
    },
    math::distance::Distances,
    metrics::mean_squared_error,
    model_selection::train_test_split,
    neighbors::knn_regressor::{KNNRegressor, KNNRegressorParameters},
    svm::{
        svr::{SVRParameters, SVR},
        Kernels,
    },
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

    let (org_x, org_y) = load_data(config, mysql_cli, begin, end)?;
    if org_x.len() < config.training_data_required_count {
        warn!(
            "training data is too little. skip training. count:{}",
            org_x.len()
        );
        return Ok(());
    }

    save_training_datasets(config, mysql_cli, &org_x, &org_y)?;

    let matrix = DenseMatrix::from_2d_vec(&org_x);
    let (train_base_x, test_x, train_base_y, test_y) = train_test_split(&matrix, &org_y, 0.2, true);

    let mut models: Vec<ForecastModel> = vec![];
    if let Some(m) = load_existing_model(config, mysql_cli)? {
        let input_data_size = m.get_input_data_size()?;
        if input_data_size == config.forecast_input_size {
            models.push(m);
        } else {
            warn!(
                "input data size is not match, not use existing model. model: {}, training: {}",
                input_data_size, config.forecast_input_size
            );
        }
    }
    for index in 1..=config.training_count {
        debug!("training RandomForest {:2} ...", index);
        let (train_x, _, train_y, _) = train_test_split(&train_base_x, &train_base_y, 0.2, true);
        let m = ForecastModel::RandomForest {
            pair: config.currency_pair.clone(),
            no: config.forecast_model_no,
            model: RandomForestRegressor::fit(&train_x, &train_y, Default::default())?,
            input_data_size: config.forecast_input_size,
            memo: "RandomForest".to_string(),
        };
        models.push(m);
    }
    for index in 1..=config.training_count {
        debug!("training KNN {:2} ...", index);
        let (train_x, _, train_y, _) = train_test_split(&train_base_x, &train_base_y, 0.2, true);
        let r = KNNRegressor::fit(
            &train_x,
            &train_y,
            KNNRegressorParameters::default().with_distance(Distances::euclidian()),
        )?;
        let m = ForecastModel::KNN {
            pair: config.currency_pair.clone(),
            no: config.forecast_model_no,
            model: r,
            input_data_size: config.forecast_input_size,
            memo: "KNN".to_string(),
        };
        models.push(m);
    }
    for index in 1..=config.training_count {
        debug!("training Linear {:2} ...", index);
        let (train_x, _, train_y, _) = train_test_split(&train_base_x, &train_base_y, 0.2, true);
        let r = LinearRegression::fit(&train_x, &train_y, Default::default())?;
        let m = ForecastModel::Linear {
            pair: config.currency_pair.clone(),
            no: config.forecast_model_no,
            model: r,
            input_data_size: config.forecast_input_size,
            memo: "Linear".to_string(),
        };
        models.push(m);
    }
    for index in 1..=config.training_count {
        debug!("training Ridge {:2} ...", index);
        let (train_x, _, train_y, _) = train_test_split(&train_base_x, &train_base_y, 0.2, true);
        let r = RidgeRegression::fit(
            &train_x,
            &train_y,
            RidgeRegressionParameters::default().with_alpha(0.5),
        )?;
        let m = ForecastModel::Ridge {
            pair: config.currency_pair.clone(),
            no: config.forecast_model_no,
            model: r,
            input_data_size: config.forecast_input_size,
            memo: "Ridge".to_string(),
        };
        models.push(m);
    }
    for index in 1..=config.training_count {
        debug!("training LASSO {:2} ...", index);
        let (train_x, _, train_y, _) = train_test_split(&train_base_x, &train_base_y, 0.2, true);
        let r = Lasso::fit(
            &train_x,
            &train_y,
            LassoParameters::default().with_alpha(0.5),
        )?;
        let m = ForecastModel::LASSO {
            pair: config.currency_pair.clone(),
            no: config.forecast_model_no,
            model: r,
            input_data_size: config.forecast_input_size,
            memo: "LASSO".to_string(),
        };
        models.push(m);
    }
    for index in 1..=config.training_count {
        debug!("training ElasticNet {:2} ...", index);
        let (train_x, _, train_y, _) = train_test_split(&train_base_x, &train_base_y, 0.2, true);
        let r = ElasticNet::fit(
            &train_x,
            &train_y,
            ElasticNetParameters::default()
                .with_alpha(0.5)
                .with_l1_ratio(0.5),
        )?;
        let m = ForecastModel::ElasticNet {
            pair: config.currency_pair.clone(),
            no: config.forecast_model_no,
            model: r,
            input_data_size: config.forecast_input_size,
            memo: "ElasticNet".to_string(),
        };
        models.push(m);
    }
    // 学習が終わらなかったためコメントアウト
    // for index in 1..=config.training_count {
    //     debug!("training Logistic {:2} ...", index);
    //     let (train_x, _, train_y, _) = train_test_split(&train_base_x, &train_base_y, 0.2, true);
    //     let r = LogisticRegression::fit(
    //         &train_x,
    //         &train_y,
    //         Default::default(),
    //     )?;
    //     let m = ForecastModel::Logistic {
    //         pair: config.currency_pair.clone(),
    //         no: config.forecast_model_no,
    //         model: r,
    //         memo: "Logistic".to_string(),
    //     };
    //     models.push(m);
    // }
    for index in 1..=config.training_count {
        debug!("training SVR {:2} ...", index);
        let (train_x, _, train_y, _) = train_test_split(&train_base_x, &train_base_y, 0.2, true);
        let r = SVR::fit(
            &train_x,
            &train_y,
            SVRParameters::default()
                .with_kernel(Kernels::rbf(0.5))
                .with_c(2000.0)
                .with_eps(10.0),
        )?;
        let m = ForecastModel::SVR {
            pair: config.currency_pair.clone(),
            no: config.forecast_model_no,
            model: r,
            input_data_size: config.forecast_input_size,
            memo: "SVR".to_string(),
        };
        models.push(m);
    }

    let mut best_model: Option<&ForecastModel> = None;
    let mut best_mne: Option<f64> = None;
    for m in models.iter() {
        let y = m.predict_for_training(&test_x)?;

        let mne = mean_squared_error(&test_y, &y);
        info!("MSE: {:.6}, model: {}", mne, m);

        if best_mne.is_none() || mne < best_mne.unwrap() {
            best_model = Some(m);
            best_mne = Some(mne);
        }
    }

    if let Some(mne) = best_mne {
        info!("Best MSE: {:6}, model: {}", mne, best_model.unwrap());
    } else {
        error!("model not found");
        return Ok(());
    }

    let best_model = best_model.unwrap();
    let y = best_model.predict_for_training(&test_x)?;
    for row in 0..test_y.len() {
        let want = test_y[row];
        let got = y[row];
        let diff = want - got;
        info!(
            "[no{:03}] want: {:.4}, got: {:.4}, diff: {:.4}",
            row + 1,
            want,
            got,
            diff
        );
    }

    save_model(mysql_cli, &best_model)?;

    Ok(())
}

fn load_data(
    config: &config::Config,
    mysql_cli: &DefaultClient,
    begin: NaiveDateTime,
    end: NaiveDateTime,
) -> MyResult<(Vec<Vec<f64>>, Vec<f64>)> {
    let mut x: Vec<Vec<f64>> = vec![];
    let mut y: Vec<f64> = vec![];

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
            x.push(data);
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

fn load_existing_model(
    config: &config::Config,
    mysql_cli: &DefaultClient,
) -> MyResult<Option<ForecastModel>> {
    let mut model: Option<ForecastModel> = None;
    mysql_cli.with_transaction(|tx| -> MyResult<()> {
        model =
            mysql_cli.select_forecast_model(tx, &config.currency_pair, config.forecast_model_no)?;
        Ok(())
    })?;
    Ok(model)
}

fn save_model(mysql_cli: &DefaultClient, model: &ForecastModel) -> MyResult<()> {
    mysql_cli.with_transaction(|tx| {
        mysql_cli.upsert_forecast_model(tx, model)?;
        Ok(())
    })?;
    Ok(())
}
