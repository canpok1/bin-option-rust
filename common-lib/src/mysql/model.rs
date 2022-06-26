use chrono::NaiveDate;
use smartcore::{
    ensemble::random_forest_regressor::RandomForestRegressor, neighbors::knn_regressor::KNNRegressor, math::distance::euclidian, linear::{linear_regression::LinearRegression, ridge_regression::RidgeRegression, lasso::Lasso, elastic_net::ElasticNet, logistic_regression::LogisticRegression}, linalg::naive::dense_matrix::DenseMatrix, svm::{RBFKernel, svr::SVR}
};

use crate::{error::{MyResult, MyError}, domain};

pub const MODEL_TYPE_RANDOM_FOREST:u8 = 0;
pub const MODEL_TYPE_KNN:u8 = 1;
pub const MODEL_TYPE_LINEAR:u8 = 2;
pub const MODEL_TYPE_RIDGE:u8 = 3;
pub const MODEL_TYPE_LASSO:u8 = 4;
pub const MODEL_TYPE_ELASTIC_NET:u8 = 5;
pub const MODEL_TYPE_LOGISTIC:u8 = 6;
pub const MODEL_TYPE_SVR:u8 = 7;

#[derive(Debug, Clone)]
pub struct ForecastModelRecord {
    pub pair: String,
    pub model_no: i32,
    pub model_type: u8,
    pub model_data: Vec<u8>,
    pub memo: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl ForecastModelRecord
{
    pub fn new(pair: String, model_no: i32, model_type: u8, model_data: Vec<u8>, memo: String) -> MyResult<ForecastModelRecord> {
        let dummy = NaiveDate::from_ymd(2022, 1, 1).and_hms(0, 0, 0);
        Ok(ForecastModelRecord {
            pair,
            model_no,
            model_type,
            model_data,
            memo,
            created_at: dummy.clone(),
            updated_at: dummy.clone(),
        })
    }

    pub fn to_domain(&self) -> MyResult<domain::model::ForecastModel> {
        match self.model_type {
            MODEL_TYPE_RANDOM_FOREST => {
                Ok(domain::model::ForecastModel::RandomForest {
                    pair: self.pair.clone(),
                    no: self.model_no,
                    model: bincode::deserialize::<RandomForestRegressor<f64>>(&self.model_data)?,
                    memo: self.memo.clone(),
                })
            },
            MODEL_TYPE_KNN => {
                Ok(domain::model::ForecastModel::KNN {
                    pair: self.pair.clone(),
                    no: self.model_no,
                    model: bincode::deserialize::<KNNRegressor<f64, euclidian::Euclidian>>(&self.model_data)?,
                    memo: self.memo.clone(),
                })
            },
            MODEL_TYPE_LINEAR => {
                Ok(domain::model::ForecastModel::Linear {
                    pair: self.pair.clone(),
                    no: self.model_no,
                    model: bincode::deserialize::<LinearRegression<f64, DenseMatrix<f64>>>(&self.model_data)?,
                    memo: self.memo.clone(),
                })
            },
            MODEL_TYPE_RIDGE => {
                Ok(domain::model::ForecastModel::Ridge {
                    pair: self.pair.clone(),
                    no: self.model_no,
                    model: bincode::deserialize::<RidgeRegression<f64, DenseMatrix<f64>>>(&self.model_data)?,
                    memo: self.memo.clone(),
                })
            },
            MODEL_TYPE_LASSO => {
                Ok(domain::model::ForecastModel::LASSO {
                    pair: self.pair.clone(),
                    no: self.model_no,
                    model: bincode::deserialize::<Lasso<f64, DenseMatrix<f64>>>(&self.model_data)?,
                    memo: self.memo.clone(),
                })
            },
            MODEL_TYPE_ELASTIC_NET => {
                Ok(domain::model::ForecastModel::ElasticNet {
                    pair: self.pair.clone(),
                    no: self.model_no,
                    model: bincode::deserialize::<ElasticNet<f64, DenseMatrix<f64>>>(&self.model_data)?,
                    memo: self.memo.clone(),
                })
            },
            MODEL_TYPE_LOGISTIC => {
                Ok(domain::model::ForecastModel::Logistic {
                    pair: self.pair.clone(),
                    no: self.model_no,
                    model: bincode::deserialize::<LogisticRegression<f64, DenseMatrix<f64>>>(&self.model_data)?,
                    memo: self.memo.clone(),
                })
            },
            MODEL_TYPE_SVR => {
                Ok(domain::model::ForecastModel::SVR {
                    pair: self.pair.clone(),
                    no: self.model_no,
                    model: bincode::deserialize::<SVR<f64, DenseMatrix<f64>, RBFKernel<f64>>>(&self.model_data)?,
                    memo: self.memo.clone(),
                })
            },
            _ => {
                Err(Box::new(MyError::UnknownModelType { value: self.model_type }))
            }
        }
    }
}
