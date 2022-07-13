use std::fmt;

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use smartcore::{
    ensemble::random_forest_regressor::RandomForestRegressor,
    linalg::naive::dense_matrix::DenseMatrix,
    linear::{
        elastic_net::ElasticNet, lasso::Lasso, linear_regression::LinearRegression,
        logistic_regression::LogisticRegression, ridge_regression::RidgeRegression,
    },
    math::distance::euclidian,
    metrics::mean_squared_error,
    neighbors::knn_regressor::KNNRegressor,
    svm::{svr::SVR, RBFKernel},
};

use crate::error::{MyError, MyResult};

pub type InputData = Vec<f64>;
pub type FeatureData = Vec<f64>;

#[derive(Debug, Clone)]
pub struct RateForTraining {
    pub pair: String,
    pub recorded_at: chrono::NaiveDateTime,
    pub rate: f64,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl RateForTraining {
    pub fn new(pair: &str, time: &str, rate: f64) -> MyResult<RateForTraining> {
        let recored_at: NaiveDateTime;
        match NaiveDateTime::parse_from_str(time, "%Y-%m-%d %H:%M:%S") {
            Ok(v) => {
                recored_at = v;
            }
            Err(err) => {
                return Err(Box::new(MyError::ParseError {
                    param_name: "time".to_string(),
                    value: time.to_string(),
                    memo: format!("{}", err),
                }));
            }
        }
        Ok(RateForTraining {
            pair: pair.to_string(),
            recorded_at: recored_at,
            rate: rate,
            created_at: NaiveDate::from_ymd(2022, 1, 1).and_hms(0, 0, 0),
            updated_at: NaiveDate::from_ymd(2022, 1, 1).and_hms(0, 0, 0),
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FeatureParams {
    pub feature_size: usize,
    pub fast_period: usize,
    pub slow_period: usize,
    pub signal_period: usize,
    pub bb_period: usize,
}

impl FeatureParams {
    pub fn new_default() -> FeatureParams {
        FeatureParams {
            feature_size: 10,
            fast_period: 3,
            slow_period: 6,
            signal_period: 4,
            bb_period: 3,
        }
    }

    pub fn to_hash(&self) -> MyResult<String> {
        let s = format!("{:?}", self);

        let mut hasher = Sha256::new();
        hasher.update(s.as_bytes());
        let hash = hasher.finalize();

        Ok(format!("{:02x}", hash))
    }
}

pub enum ForecastModel {
    RandomForest {
        pair: String,
        no: i32,
        model: RandomForestRegressor<f64>,
        input_data_size: usize,
        feature_params: FeatureParams,
        performance_mse: f64,
        performance_rmse: f64,
        memo: String,
    },
    KNN {
        pair: String,
        no: i32,
        model: KNNRegressor<f64, euclidian::Euclidian>,
        input_data_size: usize,
        feature_params: FeatureParams,
        performance_mse: f64,
        performance_rmse: f64,
        memo: String,
    },
    Linear {
        pair: String,
        no: i32,
        model: LinearRegression<f64, DenseMatrix<f64>>,
        input_data_size: usize,
        feature_params: FeatureParams,
        performance_mse: f64,
        performance_rmse: f64,
        memo: String,
    },
    Ridge {
        pair: String,
        no: i32,
        model: RidgeRegression<f64, DenseMatrix<f64>>,
        input_data_size: usize,
        feature_params: FeatureParams,
        performance_mse: f64,
        performance_rmse: f64,
        memo: String,
    },
    LASSO {
        pair: String,
        no: i32,
        model: Lasso<f64, DenseMatrix<f64>>,
        input_data_size: usize,
        feature_params: FeatureParams,
        performance_mse: f64,
        performance_rmse: f64,
        memo: String,
    },
    ElasticNet {
        pair: String,
        no: i32,
        model: ElasticNet<f64, DenseMatrix<f64>>,
        input_data_size: usize,
        feature_params: FeatureParams,
        performance_mse: f64,
        performance_rmse: f64,
        memo: String,
    },
    Logistic {
        pair: String,
        no: i32,
        model: LogisticRegression<f64, DenseMatrix<f64>>,
        input_data_size: usize,
        feature_params: FeatureParams,
        performance_mse: f64,
        performance_rmse: f64,
        memo: String,
    },
    SVR {
        pair: String,
        no: i32,
        model: SVR<f64, DenseMatrix<f64>, RBFKernel<f64>>,
        input_data_size: usize,
        feature_params: FeatureParams,
        performance_mse: f64,
        performance_rmse: f64,
        memo: String,
    },
}

impl ForecastModel {
    pub fn get_pair(&self) -> MyResult<String> {
        match self {
            ForecastModel::RandomForest { pair, .. } => Ok(pair.to_string()),
            ForecastModel::KNN { pair, .. } => Ok(pair.to_string()),
            ForecastModel::Linear { pair, .. } => Ok(pair.to_string()),
            ForecastModel::Ridge { pair, .. } => Ok(pair.to_string()),
            ForecastModel::LASSO { pair, .. } => Ok(pair.to_string()),
            ForecastModel::ElasticNet { pair, .. } => Ok(pair.to_string()),
            ForecastModel::Logistic { pair, .. } => Ok(pair.to_string()),
            ForecastModel::SVR { pair, .. } => Ok(pair.to_string()),
        }
    }

    pub fn get_no(&self) -> MyResult<i32> {
        match self {
            ForecastModel::RandomForest { no, .. } => Ok(*no),
            ForecastModel::KNN { no, .. } => Ok(*no),
            ForecastModel::Linear { no, .. } => Ok(*no),
            ForecastModel::Ridge { no, .. } => Ok(*no),
            ForecastModel::LASSO { no, .. } => Ok(*no),
            ForecastModel::ElasticNet { no, .. } => Ok(*no),
            ForecastModel::Logistic { no, .. } => Ok(*no),
            ForecastModel::SVR { no, .. } => Ok(*no),
        }
    }

    pub fn get_input_data_size(&self) -> MyResult<usize> {
        match self {
            ForecastModel::RandomForest {
                input_data_size, ..
            } => Ok(*input_data_size),
            ForecastModel::KNN {
                input_data_size, ..
            } => Ok(*input_data_size),
            ForecastModel::Linear {
                input_data_size, ..
            } => Ok(*input_data_size),
            ForecastModel::Ridge {
                input_data_size, ..
            } => Ok(*input_data_size),
            ForecastModel::LASSO {
                input_data_size, ..
            } => Ok(*input_data_size),
            ForecastModel::ElasticNet {
                input_data_size, ..
            } => Ok(*input_data_size),
            ForecastModel::Logistic {
                input_data_size, ..
            } => Ok(*input_data_size),
            ForecastModel::SVR {
                input_data_size, ..
            } => Ok(*input_data_size),
        }
    }

    pub fn get_feature_params(&self) -> MyResult<FeatureParams> {
        match self {
            ForecastModel::RandomForest { feature_params, .. } => Ok(feature_params.clone()),
            ForecastModel::KNN { feature_params, .. } => Ok(feature_params.clone()),
            ForecastModel::Linear { feature_params, .. } => Ok(feature_params.clone()),
            ForecastModel::Ridge { feature_params, .. } => Ok(feature_params.clone()),
            ForecastModel::LASSO { feature_params, .. } => Ok(feature_params.clone()),
            ForecastModel::ElasticNet { feature_params, .. } => Ok(feature_params.clone()),
            ForecastModel::Logistic { feature_params, .. } => Ok(feature_params.clone()),
            ForecastModel::SVR { feature_params, .. } => Ok(feature_params.clone()),
        }
    }

    pub fn get_performance_mse(&self) -> MyResult<f64> {
        match self {
            ForecastModel::RandomForest {
                performance_mse, ..
            } => Ok(*performance_mse),
            ForecastModel::KNN {
                performance_mse, ..
            } => Ok(*performance_mse),
            ForecastModel::Linear {
                performance_mse, ..
            } => Ok(*performance_mse),
            ForecastModel::Ridge {
                performance_mse, ..
            } => Ok(*performance_mse),
            ForecastModel::LASSO {
                performance_mse, ..
            } => Ok(*performance_mse),
            ForecastModel::ElasticNet {
                performance_mse, ..
            } => Ok(*performance_mse),
            ForecastModel::Logistic {
                performance_mse, ..
            } => Ok(*performance_mse),
            ForecastModel::SVR {
                performance_mse, ..
            } => Ok(*performance_mse),
        }
    }

    fn set_performance_mse(&mut self, v: f64) -> MyResult<()> {
        match self {
            ForecastModel::RandomForest {
                performance_mse,
                performance_rmse,
                ..
            } => {
                *performance_mse = v;
                *performance_rmse = v.sqrt();
            }
            ForecastModel::KNN {
                performance_mse,
                performance_rmse,
                ..
            } => {
                *performance_mse = v;
                *performance_rmse = v.sqrt();
            }
            ForecastModel::Linear {
                performance_mse,
                performance_rmse,
                ..
            } => {
                *performance_mse = v;
                *performance_rmse = v.sqrt();
            }
            ForecastModel::Ridge {
                performance_mse,
                performance_rmse,
                ..
            } => {
                *performance_mse = v;
                *performance_rmse = v.sqrt();
            }
            ForecastModel::LASSO {
                performance_mse,
                performance_rmse,
                ..
            } => {
                *performance_mse = v;
                *performance_rmse = v.sqrt();
            }
            ForecastModel::ElasticNet {
                performance_mse,
                performance_rmse,
                ..
            } => {
                *performance_mse = v;
                *performance_rmse = v.sqrt();
            }
            ForecastModel::Logistic {
                performance_mse,
                performance_rmse,
                ..
            } => {
                *performance_mse = v;
                *performance_rmse = v.sqrt();
            }
            ForecastModel::SVR {
                performance_mse,
                performance_rmse,
                ..
            } => {
                *performance_mse = v;
                *performance_rmse = v.sqrt();
            }
        }
        Ok(())
    }

    pub fn update_performance(
        &mut self,
        test_x: &Vec<FeatureData>,
        test_y: &Vec<f64>,
    ) -> MyResult<()> {
        let matrix = DenseMatrix::from_2d_vec(test_x);
        let y = self.predict_for_training(&matrix)?;
        let mse = mean_squared_error(test_y, &y);
        self.set_performance_mse(mse)?;
        Ok(())
    }

    fn predict_for_training(&self, x: &DenseMatrix<f64>) -> MyResult<Vec<f64>> {
        match self {
            ForecastModel::RandomForest { model, .. } => Ok(model.predict(x)?),
            ForecastModel::KNN { model, .. } => Ok(model.predict(x)?),
            ForecastModel::Linear { model, .. } => Ok(model.predict(x)?),
            ForecastModel::Ridge { model, .. } => Ok(model.predict(x)?),
            ForecastModel::LASSO { model, .. } => Ok(model.predict(x)?),
            ForecastModel::ElasticNet { model, .. } => Ok(model.predict(x)?),
            ForecastModel::Logistic { model, .. } => Ok(model.predict(x)?),
            ForecastModel::SVR { model, .. } => Ok(model.predict(x)?),
        }
    }

    pub fn predict(&self, rates: &FeatureData) -> MyResult<f64> {
        let org_x: Vec<FeatureData> = vec![rates.clone()];
        let x = DenseMatrix::from_2d_vec(&org_x);
        let y = self.predict_for_training(&x)?;
        Ok(y[0])
    }

    pub fn serialize_model_data(&self) -> MyResult<Vec<u8>> {
        match self {
            ForecastModel::RandomForest { model, .. } => Ok(bincode::serialize(&model)?),
            ForecastModel::KNN { model, .. } => Ok(bincode::serialize(&model)?),
            ForecastModel::Linear { model, .. } => Ok(bincode::serialize(&model)?),
            ForecastModel::Ridge { model, .. } => Ok(bincode::serialize(&model)?),
            ForecastModel::LASSO { model, .. } => Ok(bincode::serialize(&model)?),
            ForecastModel::ElasticNet { model, .. } => Ok(bincode::serialize(&model)?),
            ForecastModel::Logistic { model, .. } => Ok(bincode::serialize(&model)?),
            ForecastModel::SVR { model, .. } => Ok(bincode::serialize(&model)?),
        }
    }
}

impl fmt::Display for ForecastModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ForecastModel::RandomForest {
                pair,
                no,
                feature_params,
                performance_mse,
                performance_rmse,
                memo,
                ..
            } => {
                write!(
                    f,
                    "RandomForest(pair: {}, no: {}, feature_params: {:?}, mse: {}, rmse: {}, memo: {})",
                    pair, no, feature_params, performance_mse, performance_rmse,memo
                )
            }
            ForecastModel::KNN {
                pair,
                no,
                feature_params,
                performance_mse,
                performance_rmse,
                memo,
                ..
            } => {
                write!(
                    f,
                    "KNN(pair: {}, no: {}, feature_params: {:?}, mse: {}, rmse: {}, memo: {})",
                    pair, no, feature_params, performance_mse, performance_rmse, memo
                )
            }
            ForecastModel::Linear {
                pair,
                no,
                feature_params,
                performance_mse,
                performance_rmse,
                memo,
                ..
            } => {
                write!(
                    f,
                    "Linear(pair: {}, no: {}, feature_params: {:?}, mse: {}, rmse: {}, memo: {})",
                    pair, no, feature_params, performance_mse, performance_rmse, memo
                )
            }
            ForecastModel::Ridge {
                pair,
                no,
                feature_params,
                performance_mse,
                performance_rmse,
                memo,
                ..
            } => {
                write!(
                    f,
                    "Ridge(pair: {}, no: {}, feature_params: {:?}, mse: {}, rmse: {}, memo: {})",
                    pair, no, feature_params, performance_mse, performance_rmse, memo
                )
            }
            ForecastModel::LASSO {
                pair,
                no,
                feature_params,
                performance_mse,
                performance_rmse,
                memo,
                ..
            } => {
                write!(
                    f,
                    "LASSO(pair: {}, no: {}, feature_params: {:?}, mse: {}, rmse: {}, memo: {})",
                    pair, no, feature_params, performance_mse, performance_rmse, memo
                )
            }
            ForecastModel::ElasticNet {
                pair,
                no,
                feature_params,
                performance_mse,
                performance_rmse,
                memo,
                ..
            } => {
                write!(
                    f,
                    "ElasticNet(pair: {}, no: {}, feature_params: {:?}, mse: {}, rmse: {}, memo: {})",
                    pair, no, feature_params, performance_mse, performance_rmse, memo
                )
            }
            ForecastModel::Logistic {
                pair,
                no,
                feature_params,
                performance_mse,
                performance_rmse,
                memo,
                ..
            } => {
                write!(
                    f,
                    "Logistic(pair: {}, no: {}, feature_params: {:?}, mse: {}, rmse: {}, memo: {})",
                    pair, no, feature_params, performance_mse, performance_rmse, memo
                )
            }
            ForecastModel::SVR {
                pair,
                no,
                feature_params,
                performance_mse,
                performance_rmse,
                memo,
                ..
            } => {
                write!(
                    f,
                    "SVR(pair: {}, no: {}, feature_params: {:?}, mse: {}, rmse: {}, memo: {})",
                    pair, no, feature_params, performance_mse, performance_rmse, memo
                )
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ForecastResult {
    pub id: String,
    pub rate_id: String,
    pub model_no: i32,
    pub forecast_type: i32,
    pub result: f64,
    pub memo: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl ForecastResult {
    pub fn new(
        rate_id: String,
        model_no: i32,
        forecast_type: i32,
        result: f64,
        memo: String,
    ) -> MyResult<Self> {
        let dummy = NaiveDate::from_ymd(2022, 1, 1).and_hms(0, 0, 0);

        Ok(ForecastResult {
            id: "".to_string(),
            rate_id,
            model_no,
            forecast_type,
            result,
            memo: Some(memo),
            created_at: dummy.clone(),
            updated_at: dummy.clone(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ForecastError {
    pub id: String,
    pub rate_id: String,
    pub model_no: i32,
    pub summary: String,
    pub detail: String,
}

impl ForecastError {
    pub fn new(rate_id: String, model_no: i32, summary: String, detail: String) -> MyResult<Self> {
        Ok(ForecastError {
            id: "".to_string(),
            rate_id,
            model_no,
            summary,
            detail,
        })
    }
}
impl fmt::Display for ForecastError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}, {}, rate_id: {}, model_no: {}",
            self.summary, self.detail, self.rate_id, self.model_no
        )
    }
}

#[derive(Debug, Clone)]
pub struct RateForForecast {
    pub id: String,
    pub pair: String,
    pub histories: Vec<f64>,
    pub expire: chrono::NaiveDateTime,
    pub memo: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl RateForForecast {
    pub fn new(
        pair: String,
        histories: Vec<f64>,
        expire: NaiveDateTime,
        memo: String,
    ) -> MyResult<Self> {
        Ok(RateForForecast {
            id: "".to_string(),
            pair: pair.to_string(),
            histories: histories,
            expire: expire,
            memo: memo,
            created_at: NaiveDate::from_ymd(2022, 1, 1).and_hms(0, 0, 0),
            updated_at: NaiveDate::from_ymd(2022, 1, 1).and_hms(0, 0, 0),
        })
    }
}

#[derive(Debug, Clone)]
pub struct TrainingDataset {
    pub id: String,
    pub pair: String,
    pub input_data: Vec<f64>,
    pub truth: f64,
    pub memo: String,
}

impl TrainingDataset {
    pub fn new(pair: String, input_data: Vec<f64>, truth: f64, memo: String) -> MyResult<Self> {
        Ok(TrainingDataset {
            id: "".to_string(),
            pair: pair.to_string(),
            input_data: input_data,
            truth: truth,
            memo: memo,
        })
    }
}
