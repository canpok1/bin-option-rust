use serde::{Deserialize, Serialize};
use smartcore::{
    ensemble::random_forest_regressor::RandomForestRegressor,
    linalg::naive::dense_matrix::DenseMatrix,
    linear::{
        elastic_net::ElasticNet, lasso::Lasso, linear_regression::LinearRegression,
        logistic_regression::LogisticRegression, ridge_regression::RidgeRegression,
    },
    math::distance::euclidian,
    neighbors::knn_regressor::KNNRegressor,
    svm::{svr::SVR, RBFKernel},
};

use crate::{
    domain::{self, model::FeatureParams},
    error::{MyError, MyResult},
};

pub const MODEL_TYPE_RANDOM_FOREST: u8 = 0;
pub const MODEL_TYPE_KNN: u8 = 1;
pub const MODEL_TYPE_LINEAR: u8 = 2;
pub const MODEL_TYPE_RIDGE: u8 = 3;
pub const MODEL_TYPE_LASSO: u8 = 4;
pub const MODEL_TYPE_ELASTIC_NET: u8 = 5;
pub const MODEL_TYPE_LOGISTIC: u8 = 6;
pub const MODEL_TYPE_SVR: u8 = 7;

#[derive(Debug, Clone)]
pub struct ForecastModelRecord {
    pub pair: String,
    pub model_no: i32,
    pub model_type: u8,
    pub model_data: Vec<u8>,
    pub input_data_size: usize,
    pub feature_params: FeatureParams,
    pub feature_params_hash: String,
    pub performance_mse: f64,
    pub performance_rmse: f64,
    pub memo: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl ForecastModelRecord {
    pub fn validate_feature_params(&self) -> MyResult<()> {
        if self.feature_params.to_hash()? == self.feature_params_hash {
            return Ok(());
        }
        Err(Box::new(MyError::UnmatchFeatureParamsHash {
            pair: self.pair.to_string(),
            model_no: self.model_no,
        }))
    }

    pub fn to_domain(&self) -> MyResult<domain::model::ForecastModel> {
        match self.model_type {
            MODEL_TYPE_RANDOM_FOREST => Ok(domain::model::ForecastModel::RandomForest {
                pair: self.pair.clone(),
                no: self.model_no,
                model: bincode::deserialize::<RandomForestRegressor<f64>>(&self.model_data)?,
                input_data_size: self.input_data_size,
                feature_params: self.feature_params.clone(),
                performance_mse: self.performance_mse,
                performance_rmse: self.performance_rmse,
                memo: self.memo.clone(),
            }),
            MODEL_TYPE_KNN => Ok(domain::model::ForecastModel::KNN {
                pair: self.pair.clone(),
                no: self.model_no,
                model: bincode::deserialize::<KNNRegressor<f64, euclidian::Euclidian>>(
                    &self.model_data,
                )?,
                input_data_size: self.input_data_size,
                feature_params: self.feature_params.clone(),
                performance_mse: self.performance_mse,
                performance_rmse: self.performance_rmse,
                memo: self.memo.clone(),
            }),
            MODEL_TYPE_LINEAR => Ok(domain::model::ForecastModel::Linear {
                pair: self.pair.clone(),
                no: self.model_no,
                model: bincode::deserialize::<LinearRegression<f64, DenseMatrix<f64>>>(
                    &self.model_data,
                )?,
                input_data_size: self.input_data_size,
                feature_params: self.feature_params.clone(),
                performance_mse: self.performance_mse,
                performance_rmse: self.performance_rmse,
                memo: self.memo.clone(),
            }),
            MODEL_TYPE_RIDGE => Ok(domain::model::ForecastModel::Ridge {
                pair: self.pair.clone(),
                no: self.model_no,
                model: bincode::deserialize::<RidgeRegression<f64, DenseMatrix<f64>>>(
                    &self.model_data,
                )?,
                input_data_size: self.input_data_size,
                feature_params: self.feature_params.clone(),
                performance_mse: self.performance_mse,
                performance_rmse: self.performance_rmse,
                memo: self.memo.clone(),
            }),
            MODEL_TYPE_LASSO => Ok(domain::model::ForecastModel::LASSO {
                pair: self.pair.clone(),
                no: self.model_no,
                model: bincode::deserialize::<Lasso<f64, DenseMatrix<f64>>>(&self.model_data)?,
                input_data_size: self.input_data_size,
                feature_params: self.feature_params.clone(),
                performance_mse: self.performance_mse,
                performance_rmse: self.performance_rmse,
                memo: self.memo.clone(),
            }),
            MODEL_TYPE_ELASTIC_NET => Ok(domain::model::ForecastModel::ElasticNet {
                pair: self.pair.clone(),
                no: self.model_no,
                model: bincode::deserialize::<ElasticNet<f64, DenseMatrix<f64>>>(&self.model_data)?,
                input_data_size: self.input_data_size,
                feature_params: self.feature_params.clone(),
                performance_mse: self.performance_mse,
                performance_rmse: self.performance_rmse,
                memo: self.memo.clone(),
            }),
            MODEL_TYPE_LOGISTIC => Ok(domain::model::ForecastModel::Logistic {
                pair: self.pair.clone(),
                no: self.model_no,
                model: bincode::deserialize::<LogisticRegression<f64, DenseMatrix<f64>>>(
                    &self.model_data,
                )?,
                input_data_size: self.input_data_size,
                feature_params: self.feature_params.clone(),
                performance_mse: self.performance_mse,
                performance_rmse: self.performance_rmse,
                memo: self.memo.clone(),
            }),
            MODEL_TYPE_SVR => Ok(domain::model::ForecastModel::SVR {
                pair: self.pair.clone(),
                no: self.model_no,
                model: bincode::deserialize::<SVR<f64, DenseMatrix<f64>, RBFKernel<f64>>>(
                    &self.model_data,
                )?,
                input_data_size: self.input_data_size,
                feature_params: self.feature_params.clone(),
                performance_mse: self.performance_mse,
                performance_rmse: self.performance_rmse,
                memo: self.memo.clone(),
            }),
            _ => Err(Box::new(MyError::UnknownModelType {
                value: self.model_type,
            })),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FeatureParamsValue {
    pub feature_size: Option<usize>,
    pub fast_period: Option<usize>,
    pub slow_period: Option<usize>,
    pub signal_period: Option<usize>,
    pub bb_period: Option<usize>,
}

impl FeatureParamsValue {
    pub fn to_domain(&self) -> MyResult<domain::model::FeatureParams> {
        let mut m = FeatureParams::new_default();

        if let Some(v) = self.feature_size {
            m.feature_size = v;
        }
        if let Some(v) = self.fast_period {
            m.fast_period = v;
        }
        if let Some(v) = self.slow_period {
            m.slow_period = v;
        }
        if let Some(v) = self.signal_period {
            m.signal_period = v;
        }
        if let Some(v) = self.bb_period {
            m.bb_period = v;
        }

        Ok(m)
    }
}
