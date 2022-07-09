use std::collections::HashSet;

use chrono::{Duration, NaiveDateTime, Utc};
use common_lib::{
    batch,
    domain::{
        model::{FeatureParams, ForecastModel, TrainingDataset},
        service::Converter,
    },
    error::MyResult,
    mysql::{
        self,
        client::{Client, DefaultClient},
    },
};
use config::Config;
use log::{debug, error, info, warn};
use rand::Rng;
use smartcore::{
    ensemble::random_forest_regressor::*,
    linalg::naive::dense_matrix::DenseMatrix,
    linear::{
        elastic_net::{ElasticNet, ElasticNetParameters},
        lasso::{Lasso, LassoParameters},
        linear_regression::LinearRegression,
        ridge_regression::{RidgeRegression, RidgeRegressionParameters},
    },
    math::distance::Distances,
    metrics::mean_squared_error,
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

    save_training_datasets(config, mysql_cli, &org_x, &org_y)?;

    let (train_base_x, test_x, train_base_y, test_y) = train_test_split(&org_x, &org_y, 0.2)?;
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
        let (train_x, _, train_y, _) = train_test_split(&train_base_x, &train_base_y, 0.2)?;

        debug!("training[{:2}] RandomForest ...", index);
        models.push(make_random_forest(&p, &train_x, &train_y, config)?);

        debug!("training[{:2}] KNN ...", index);
        models.push(make_knn(&p, &train_x, &train_y, config)?);

        debug!("training[{:2}] Linear ...", index);
        models.push(make_linear(&p, &train_x, &train_y, config)?);

        debug!("training[{:2}] Ridge ...", index);
        models.push(make_ridge(&p, &train_x, &train_y, config)?);

        debug!("training[{:2}] LASSO ...", index);
        models.push(make_lasso(&p, &train_x, &train_y, config)?);

        debug!("training[{:2}] ElasticNet ...", index);
        models.push(make_elastic_net(&p, &train_x, &train_y, config)?);

        //  学習が終わらなかったためコメントアウト
        //  debug!("training[{:2}] Logistic ...", index);
        //  models.push(make_elastic_net(&p, &train_x, &train_y, config)?);

        debug!("training[{:2}] SVR ...", index);
        models.push(make_svr(&p, &train_x, &train_y, config)?);
    }

    let best_model: &ForecastModel = select_best_model(&models, &test_x, &test_y)?.unwrap();
    let matrix = DenseMatrix::from_2d_vec(&test_x);
    let y = best_model.predict_for_training(&matrix)?;
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

fn make_random_forest(
    params: &FeatureParams,
    train_x: &Vec<Vec<f64>>,
    train_y: &Vec<f64>,
    config: &Config,
) -> MyResult<ForecastModel> {
    let matrix = DenseMatrix::from_2d_vec(&train_x);
    Ok(ForecastModel::RandomForest {
        pair: config.currency_pair.clone(),
        no: config.forecast_model_no,
        model: RandomForestRegressor::fit(&matrix, &train_y, Default::default())?,
        input_data_size: config.forecast_input_size,
        feature_params: params.clone(),
        memo: "RandomForest".to_string(),
    })
}

fn make_knn(
    params: &FeatureParams,
    train_x: &Vec<Vec<f64>>,
    train_y: &Vec<f64>,
    config: &Config,
) -> MyResult<ForecastModel> {
    let matrix = DenseMatrix::from_2d_vec(&train_x);
    let r = KNNRegressor::fit(
        &matrix,
        &train_y,
        KNNRegressorParameters::default().with_distance(Distances::euclidian()),
    )?;
    Ok(ForecastModel::KNN {
        pair: config.currency_pair.clone(),
        no: config.forecast_model_no,
        model: r,
        input_data_size: config.forecast_input_size,
        feature_params: params.clone(),
        memo: "KNN".to_string(),
    })
}

fn make_linear(
    params: &FeatureParams,
    train_x: &Vec<Vec<f64>>,
    train_y: &Vec<f64>,
    config: &Config,
) -> MyResult<ForecastModel> {
    let matrix = DenseMatrix::from_2d_vec(&train_x);
    let r = LinearRegression::fit(&matrix, &train_y, Default::default())?;
    Ok(ForecastModel::Linear {
        pair: config.currency_pair.clone(),
        no: config.forecast_model_no,
        model: r,
        input_data_size: config.forecast_input_size,
        feature_params: params.clone(),
        memo: "Linear".to_string(),
    })
}

fn make_ridge(
    params: &FeatureParams,
    train_x: &Vec<Vec<f64>>,
    train_y: &Vec<f64>,
    config: &Config,
) -> MyResult<ForecastModel> {
    let matrix = DenseMatrix::from_2d_vec(&train_x);
    let r = RidgeRegression::fit(
        &matrix,
        &train_y,
        RidgeRegressionParameters::default().with_alpha(0.5),
    )?;
    Ok(ForecastModel::Ridge {
        pair: config.currency_pair.clone(),
        no: config.forecast_model_no,
        model: r,
        input_data_size: config.forecast_input_size,
        feature_params: params.clone(),
        memo: "Ridge".to_string(),
    })
}
fn make_lasso(
    params: &FeatureParams,
    train_x: &Vec<Vec<f64>>,
    train_y: &Vec<f64>,
    config: &Config,
) -> MyResult<ForecastModel> {
    let matrix = DenseMatrix::from_2d_vec(&train_x);
    let r = Lasso::fit(
        &matrix,
        &train_y,
        LassoParameters::default().with_alpha(0.5),
    )?;
    Ok(ForecastModel::LASSO {
        pair: config.currency_pair.clone(),
        no: config.forecast_model_no,
        model: r,
        input_data_size: config.forecast_input_size,
        feature_params: params.clone(),
        memo: "LASSO".to_string(),
    })
}
fn make_elastic_net(
    params: &FeatureParams,
    train_x: &Vec<Vec<f64>>,
    train_y: &Vec<f64>,
    config: &Config,
) -> MyResult<ForecastModel> {
    let matrix = DenseMatrix::from_2d_vec(&train_x);
    let r = ElasticNet::fit(
        &matrix,
        &train_y,
        ElasticNetParameters::default()
            .with_alpha(0.5)
            .with_l1_ratio(0.5),
    )?;
    Ok(ForecastModel::ElasticNet {
        pair: config.currency_pair.clone(),
        no: config.forecast_model_no,
        model: r,
        input_data_size: config.forecast_input_size,
        feature_params: params.clone(),
        memo: "ElasticNet".to_string(),
    })
}

// fn make_ligistic(
//     params: &FeatureParams,
//     train_x: &Vec<Vec<f64>>,
//     train_y: &Vec<f64>,
//     config: &Config,
// ) -> MyResult<ForecastModel> {
//     let matrix = DenseMatrix::from_2d_vec(&train_x);
//     let r = LogisticRegression::fit(
//         &train_x,
//         &train_y,
//         Default::default(),
//     )?;
//     Ok(ForecastModel::Logistic {
//         pair: config.currency_pair.clone(),
//         no: config.forecast_model_no,
//         model: r,
//         memo: "Logistic".to_string(),
//     })
// }

fn make_svr(
    params: &FeatureParams,
    train_x: &Vec<Vec<f64>>,
    train_y: &Vec<f64>,
    config: &Config,
) -> MyResult<ForecastModel> {
    let matrix = DenseMatrix::from_2d_vec(&train_x);
    let r = SVR::fit(
        &matrix,
        &train_y,
        SVRParameters::default()
            .with_kernel(Kernels::rbf(0.5))
            .with_c(2000.0)
            .with_eps(10.0),
    )?;
    Ok(ForecastModel::SVR {
        pair: config.currency_pair.clone(),
        no: config.forecast_model_no,
        model: r,
        input_data_size: config.forecast_input_size,
        feature_params: params.clone(),
        memo: "SVR".to_string(),
    })
}

fn select_best_model<'a>(
    models: &'a Vec<ForecastModel>,
    test_x: &Vec<Vec<f64>>,
    test_y: &Vec<f64>,
) -> MyResult<Option<&'a ForecastModel>> {
    let matrix = DenseMatrix::from_2d_vec(test_x);

    let mut best_model: Option<&ForecastModel> = None;
    let mut best_mne: Option<f64> = None;
    for m in models.iter() {
        let y = m.predict_for_training(&matrix)?;

        let mne = mean_squared_error(test_y, &y);
        if best_mne.is_none() || mne < best_mne.unwrap() {
            best_model = Some(m);
            best_mne = Some(mne);
        }
    }

    if let Some(mne) = best_mne {
        let m = best_model.unwrap();
        info!("Best MSE: {:6}, model: {}", mne, m);
        Ok(Some(m))
    } else {
        error!("model not found");
        Ok(None)
    }
}

fn train_test_split(
    x: &Vec<Vec<f64>>,
    y: &Vec<f64>,
    test_ratio: f32,
) -> MyResult<(Vec<Vec<f64>>, Vec<Vec<f64>>, Vec<f64>, Vec<f64>)> {
    let mut test_indexes = HashSet::new();
    let mut rng = rand::thread_rng();

    for i in 0..x.len() {
        if rng.gen::<f32>() <= test_ratio {
            test_indexes.insert(i);
        }
    }

    let mut train_x = vec![];
    let mut train_y = vec![];
    let mut test_x = vec![];
    let mut test_y = vec![];
    for i in 0..x.len() {
        if test_indexes.contains(&i) {
            test_x.push(x[i].clone());
            test_y.push(y[i]);
        } else {
            train_x.push(x[i].clone());
            train_y.push(y[i]);
        }
    }

    Ok((train_x, test_x, train_y, test_y))
}
