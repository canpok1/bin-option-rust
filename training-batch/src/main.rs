use chrono::{Duration, Utc, NaiveDateTime};
use common_lib::{
    error::MyResult,
    mysql::{
        self,
        client::{DefaultClient, Client},
    }, domain::model::ForecastModel,
};
use job_scheduler::{JobScheduler, Job};
use log::{error, info, warn, debug};
use smartcore::{
    linalg::naive::dense_matrix::DenseMatrix,
    model_selection::train_test_split,
    ensemble::random_forest_regressor::*,
    metrics::mean_squared_error, neighbors::knn_regressor::{KNNRegressor, KNNRegressorParameters}, math::distance::Distances, linear::{linear_regression::LinearRegression, ridge_regression::{RidgeRegression, RidgeRegressionParameters}, lasso::{LassoParameters, Lasso}, elastic_net::{ElasticNet, ElasticNetParameters}, logistic_regression::LogisticRegression}, svm::{svr::{SVRParameters, SVR}, Kernels}
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

    let mut models:Vec<ForecastModel> = vec![];
    if let Some(m) = load_existing_model(config, mysql_cli)? {
        models.push(m);
    }
    for index in 1..=config.training_count {
        debug!("training RandomForest {:2} ...", index);
        let (train_x, _, train_y, _) = train_test_split(&train_base_x, &train_base_y, 0.2, true);
        let m = ForecastModel::RandomForest {
            pair: config.currency_pair.clone(),
            no: config.forecast_model_no,
            model: RandomForestRegressor::fit(&train_x, &train_y, Default::default())?,
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
            KNNRegressorParameters::default().with_distance(Distances::euclidian())
        )?;
        let m = ForecastModel::KNN {
            pair: config.currency_pair.clone(),
            no: config.forecast_model_no,
            model: r,
            memo: "KNN".to_string(),
        };
        models.push(m);
    }
    for index in 1..=config.training_count {
        debug!("training Linear {:2} ...", index);
        let (train_x, _, train_y, _) = train_test_split(&train_base_x, &train_base_y, 0.2, true);
        let r = LinearRegression::fit(
            &train_x,
            &train_y,
            Default::default(),
        )?;
        let m = ForecastModel::Linear {
            pair: config.currency_pair.clone(),
            no: config.forecast_model_no,
            model: r,
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
            ElasticNetParameters::default().with_alpha(0.5).with_l1_ratio(0.5),
        )?;
        let m = ForecastModel::ElasticNet {
            pair: config.currency_pair.clone(),
            no: config.forecast_model_no,
            model: r,
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
            SVRParameters::default().with_kernel(Kernels::rbf(0.5)).with_c(2000.0).with_eps(10.0),
        )?;
        let m = ForecastModel::SVR {
            pair: config.currency_pair.clone(),
            no: config.forecast_model_no,
            model: r,
            memo: "SVR".to_string(),
        };
        models.push(m);
    }

    let mut best_model:Option<&ForecastModel> = None;
    let mut best_mne:Option<f64> = None;
    for m in models.iter() {
        let y = m.predict(&test_x)?;

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
    let y = best_model.predict(&test_x)?;
    for row in 0..test_y.len() {
        let want = test_y[row];
        let got = y[row];
        let diff = want - got;
        info!("[no{:03}] want: {:.4}, got: {:.4}, diff: {:.4}", row+1, want, got, diff);
    }

    save_model(mysql_cli, &best_model)?;

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


fn save_model(mysql_cli: &DefaultClient, model: &ForecastModel) -> MyResult<()> {
    mysql_cli.with_transaction(|tx| {
        mysql_cli.upsert_forecast_model(tx, model)?;
        Ok(())
    })?;
    Ok(())
}
