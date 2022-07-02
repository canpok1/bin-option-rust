use std::fmt;

use chrono::{NaiveDate, NaiveDateTime};
use smartcore::{
    linalg::naive::dense_matrix::DenseMatrix,
    ensemble::random_forest_regressor::RandomForestRegressor,
    neighbors::knn_regressor::KNNRegressor,
    math::distance::euclidian,
    linear::{linear_regression::LinearRegression, ridge_regression::RidgeRegression, lasso::Lasso, elastic_net::ElasticNet, logistic_regression::LogisticRegression}, svm::{svr::SVR, RBFKernel},
};

use crate::error::{MyError, MyResult};

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

pub enum ForecastModel
{
    RandomForest {
        pair: String,
        no: i32,
        model: RandomForestRegressor<f64>,
        memo: String,
    },
    KNN {
        pair: String,
        no: i32,
        model: KNNRegressor<f64, euclidian::Euclidian>,
        memo: String,
    },
    Linear {
        pair: String,
        no: i32,
        model: LinearRegression<f64, DenseMatrix<f64>>,
        memo: String,
    },
    Ridge {
        pair: String,
        no: i32,
        model: RidgeRegression<f64, DenseMatrix<f64>>,
        memo: String,
    },
    LASSO {
        pair: String,
        no: i32,
        model: Lasso<f64, DenseMatrix<f64>>,
        memo: String,
    },
    ElasticNet {
        pair: String,
        no: i32,
        model: ElasticNet<f64, DenseMatrix<f64>>,
        memo: String,
    },
    Logistic {
        pair: String,
        no: i32,
        model: LogisticRegression<f64, DenseMatrix<f64>>,
        memo: String,
    },
    SVR {
        pair: String,
        no: i32,
        model: SVR<f64, DenseMatrix<f64>, RBFKernel<f64>>,
        memo: String,
    }
}

impl ForecastModel {
    pub fn get_pair(&self) -> MyResult<String> {
        match self {
            ForecastModel::RandomForest { pair, no: _, model: _, memo: _ } => Ok(pair.to_string()),
            ForecastModel::KNN { pair, no: _, model: _, memo: _ } => Ok(pair.to_string()),
            ForecastModel::Linear { pair, no: _, model: _, memo: _ } => Ok(pair.to_string()),
            ForecastModel::Ridge { pair, no: _, model: _, memo: _ } => Ok(pair.to_string()),
            ForecastModel::LASSO { pair, no: _, model: _, memo: _ } => Ok(pair.to_string()),
            ForecastModel::ElasticNet { pair, no: _, model: _, memo: _ } => Ok(pair.to_string()),
            ForecastModel::Logistic { pair, no: _, model: _, memo: _ } => Ok(pair.to_string()),
            ForecastModel::SVR { pair, no: _, model: _, memo: _ } => Ok(pair.to_string()),
        }
    }

    pub fn get_no(&self) -> MyResult<i32> {
        match self {
            ForecastModel::RandomForest { pair: _, no, model: _, memo: _ } => Ok(*no),
            ForecastModel::KNN { pair: _, no, model: _, memo: _ } => Ok(*no),
            ForecastModel::Linear { pair: _, no, model: _, memo: _ } => Ok(*no),
            ForecastModel::Ridge { pair: _, no, model: _, memo: _ } => Ok(*no),
            ForecastModel::LASSO { pair: _, no, model: _, memo: _ } => Ok(*no),
            ForecastModel::ElasticNet { pair: _, no, model: _, memo: _ } => Ok(*no),
            ForecastModel::Logistic { pair: _, no, model: _, memo: _ } => Ok(*no),
            ForecastModel::SVR { pair: _, no, model: _, memo: _ } => Ok(*no),
        }
    }

    pub fn predict_for_training(&self, x: &DenseMatrix<f64>) -> MyResult<Vec<f64>> {
        match self {
            ForecastModel::RandomForest { pair: _, no: _, model, memo: _ } => Ok(model.predict(x)?),
            ForecastModel::KNN { pair: _, no: _, model, memo: _ } => Ok(model.predict(x)?),
            ForecastModel::Linear { pair: _, no: _, model, memo: _ } => Ok(model.predict(x)?),
            ForecastModel::Ridge { pair: _, no: _, model, memo: _ } => Ok(model.predict(x)?),
            ForecastModel::LASSO { pair: _, no: _, model, memo: _ } => Ok(model.predict(x)?),
            ForecastModel::ElasticNet { pair: _, no: _, model, memo: _ } => Ok(model.predict(x)?),
            ForecastModel::Logistic { pair: _, no: _, model, memo: _ } => Ok(model.predict(x)?),
            ForecastModel::SVR { pair: _, no: _, model, memo: _ } => Ok(model.predict(x)?),
        }
    }

    pub fn predict(&self, rates: &Vec<f64>) -> MyResult<f64> {
        let org_x:Vec<Vec<f64>> = vec![rates.clone()];
        let x = DenseMatrix::from_2d_vec(&org_x);
        let y = self.predict_for_training(&x)?;
        Ok(y[0])
    }

    pub fn serialize_model_data(&self) -> MyResult<Vec<u8>> {
        match self {
            ForecastModel::RandomForest { pair: _, no: _, model, memo: _ } => {
                Ok(bincode::serialize(&model)?)
            },
            ForecastModel::KNN { pair: _, no: _, model, memo: _ } => {
                Ok(bincode::serialize(&model)?)
            },
            ForecastModel::Linear { pair: _, no: _, model, memo: _ } => {
                Ok(bincode::serialize(&model)?)
            },
            ForecastModel::Ridge { pair: _, no: _, model, memo: _ } => {
                Ok(bincode::serialize(&model)?)
            },
            ForecastModel::LASSO { pair: _, no: _, model, memo: _ } => {
                Ok(bincode::serialize(&model)?)
            },
            ForecastModel::ElasticNet { pair: _, no: _, model, memo: _ } => {
                Ok(bincode::serialize(&model)?)
            },
            ForecastModel::Logistic { pair: _, no: _, model, memo: _ } => {
                Ok(bincode::serialize(&model)?)
            },
            ForecastModel::SVR { pair: _, no: _, model, memo: _ } => {
                Ok(bincode::serialize(&model)?)
            },
        }
    }
}

impl fmt::Display for ForecastModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ForecastModel::RandomForest {pair, no, model:_, memo} => {
                write!(f, "RandomForest(pair: {}, no: {}, memo: {})", pair, no, memo)
            },
            ForecastModel::KNN {pair, no, model:_, memo} => {
                write!(f, "KNN(pair: {}, no: {}, memo: {})", pair, no, memo)
            },
            ForecastModel::Linear {pair, no, model:_, memo} => {
                write!(f, "Linear(pair: {}, no: {}, memo: {})", pair, no, memo)
            },
            ForecastModel::Ridge {pair, no, model:_, memo} => {
                write!(f, "Ridge(pair: {}, no: {}, memo: {})", pair, no, memo)
            },
            ForecastModel::LASSO {pair, no, model:_, memo} => {
                write!(f, "LASSO(pair: {}, no: {}, memo: {})", pair, no, memo)
            },
            ForecastModel::ElasticNet {pair, no, model:_, memo} => {
                write!(f, "ElasticNet(pair: {}, no: {}, memo: {})", pair, no, memo)
            },
            ForecastModel::Logistic {pair, no, model:_, memo} => {
                write!(f, "Logistic(pair: {}, no: {}, memo: {})", pair, no, memo)
            },
            ForecastModel::SVR {pair, no, model:_, memo} => {
                write!(f, "SVR(pair: {}, no: {}, memo: {})", pair, no, memo)
            },
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
    pub fn new(rate_id:String, model_no: i32, forecast_type: i32, result: f64, memo: String) -> MyResult<Self> {
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
    pub fn new(pair: String, histories: Vec<f64>, expire: NaiveDateTime, memo: String) -> MyResult<Self> {
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
