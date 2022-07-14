use chrono::{Duration, NaiveDateTime, Utc};
use common_lib::{
    domain::{
        model::{FeatureData, FeatureParams, ForecastModel, InputData},
        service::convert_to_features,
    },
    error::{MyError, MyResult},
    mysql::{self, client::Client},
};
use log::{debug, warn};
use smartcore::{
    ensemble::random_forest_regressor::RandomForestRegressor,
    linalg::naive::dense_matrix::DenseMatrix,
    linear::{
        elastic_net::{ElasticNet, ElasticNetParameters},
        lasso::{Lasso, LassoParameters},
        linear_regression::LinearRegression,
        ridge_regression::{RidgeRegression, RidgeRegressionParameters},
    },
    math::distance::Distances,
    neighbors::knn_regressor::{KNNRegressor, KNNRegressorParameters},
    svm::{
        svr::{SVRParameters, SVR},
        Kernels,
    },
};

use crate::{config, util};

pub struct InputDataLoader<'a> {
    pub config: &'a config::Config,
    pub mysql_cli: &'a mysql::client::DefaultClient,
}

impl InputDataLoader<'_> {
    pub fn load_training_data(&self) -> MyResult<(Vec<InputData>, Vec<f64>)> {
        let end = (Utc::now() - Duration::hours(self.config.training_data_range_end_offset_hour))
            .naive_utc();
        let begin = (Utc::now()
            - Duration::hours(self.config.training_data_range_begin_offset_hour))
        .naive_utc();

        self.load_data(begin, end, self.config.training_data_required_count)
    }

    pub fn load_test_data(&self) -> MyResult<(Vec<InputData>, Vec<f64>)> {
        let end =
            (Utc::now() - Duration::hours(self.config.test_data_range_end_offset_hour)).naive_utc();
        let begin = (Utc::now() - Duration::hours(self.config.test_data_range_begin_offset_hour))
            .naive_utc();

        self.load_data(begin, end, self.config.test_data_required_count)
    }

    fn load_data(
        &self,
        begin: NaiveDateTime,
        end: NaiveDateTime,
        required_count: usize,
    ) -> MyResult<(Vec<InputData>, Vec<f64>)> {
        let (x, y) = util::load_input_data(self.config, self.mysql_cli, begin, end)?;
        let count = x.len();
        if count < required_count {
            return Err(Box::new(MyError::InputDataIsTooLittle {
                count,
                require: required_count,
            }));
        }

        Ok((x, y))
    }
}

pub struct ModelMaker<'a> {
    pub config: &'a config::Config,
    pub mysql_cli: &'a mysql::client::DefaultClient,
    pub train_x: &'a Vec<InputData>,
    pub train_y: &'a Vec<f64>,
    pub test_x: &'a Vec<InputData>,
    pub test_y: &'a Vec<f64>,
}

impl ModelMaker<'_> {
    const PERFORMANCE_MSE_DEFAULT: f64 = 1.0;
    const PERFORMANCE_RMSE_DEFAULT: f64 = 1.0;

    pub fn load_existing_model(&self, model_no: i32) -> MyResult<Option<ForecastModel>> {
        let model = self.mysql_cli.with_transaction(|tx| {
            self.mysql_cli
                .select_forecast_model(tx, &self.config.currency_pair, model_no)
        })?;

        if let Some(mut m) = model {
            let input_data_size = m.get_input_data_size()?;
            if input_data_size == self.config.forecast_input_size {
                let test_x = convert_to_features(self.test_x, &m.get_feature_params()?)?;
                m.update_performance(&test_x, self.test_y)?;
                Ok(Some(m))
            } else {
                warn!(
                    "input data size is not match, not use existing model. model: {}, training: {}",
                    input_data_size, self.config.forecast_input_size
                );
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn make_new_models(
        &self,
        model_no: i32,
        params: &FeatureParams,
    ) -> MyResult<Vec<ForecastModel>> {
        let mut models: Vec<ForecastModel> = vec![];

        let train_x = convert_to_features(self.train_x, params)?;
        let test_x = convert_to_features(self.test_x, params)?;

        debug!("training RandomForest ...");
        match self.make_random_forest(
            model_no,
            &params,
            &train_x,
            &self.train_y,
            &test_x,
            &self.test_y,
        ) {
            Ok(m) => {
                models.push(m);
            }
            Err(err) => {
                warn!("training skip RandomForest, error occured. error:{}", err);
            }
        }

        debug!("training KNN ...");
        match self.make_knn(
            model_no,
            &params,
            &train_x,
            &self.train_y,
            &test_x,
            &self.test_y,
        ) {
            Ok(m) => {
                models.push(m);
            }
            Err(err) => {
                warn!("training skip KNN, error occured. error:{}", err);
            }
        }

        debug!("training Linear ...");
        match self.make_linear(
            model_no,
            &params,
            &train_x,
            &self.train_y,
            &test_x,
            &self.test_y,
        ) {
            Ok(m) => {
                models.push(m);
            }
            Err(err) => {
                warn!("training skip Linear, error occured. error:{}", err);
            }
        }

        debug!("training Ridge ...");
        match self.make_ridge(
            model_no,
            &params,
            &train_x,
            &self.train_y,
            &test_x,
            &self.test_y,
        ) {
            Ok(m) => {
                models.push(m);
            }
            Err(err) => {
                warn!("training skip Ridge, error occured. error:{}", err);
            }
        }

        debug!("training LASSO ...");
        match self.make_lasso(
            model_no,
            &params,
            &train_x,
            &self.train_y,
            &test_x,
            &self.test_y,
        ) {
            Ok(m) => {
                models.push(m);
            }
            Err(err) => {
                warn!("training skip LASSO, error occured. error:{}", err);
            }
        }

        debug!("training ElasticNet ...");
        match self.make_elastic_net(
            model_no,
            &params,
            &train_x,
            &self.train_y,
            &test_x,
            &self.test_y,
        ) {
            Ok(m) => {
                models.push(m);
            }
            Err(err) => {
                warn!("training skip ElasticNet, error occured. error:{}", err);
            }
        }

        debug!("training SVR ...");
        match self.make_svr(
            model_no,
            &params,
            &train_x,
            &self.train_y,
            &test_x,
            &self.test_y,
        ) {
            Ok(m) => {
                models.push(m);
            }
            Err(err) => {
                warn!("training skip SVR, error occured. error:{}", err);
            }
        }

        Ok(models)
    }

    fn make_random_forest(
        &self,
        model_no: i32,
        params: &FeatureParams,
        train_x: &Vec<FeatureData>,
        train_y: &Vec<f64>,
        test_x: &Vec<FeatureData>,
        test_y: &Vec<f64>,
    ) -> MyResult<ForecastModel> {
        let matrix = DenseMatrix::from_2d_vec(&train_x);
        let mut m = ForecastModel::RandomForest {
            pair: self.config.currency_pair.clone(),
            no: model_no,
            model: RandomForestRegressor::fit(&matrix, &train_y, Default::default())?,
            input_data_size: self.config.forecast_input_size,
            feature_params: params.clone(),
            performance_mse: Self::PERFORMANCE_MSE_DEFAULT,
            performance_rmse: Self::PERFORMANCE_RMSE_DEFAULT,
            memo: "RandomForest".to_string(),
        };

        m.update_performance(test_x, test_y)?;

        Ok(m)
    }

    fn make_knn(
        &self,
        model_no: i32,
        params: &FeatureParams,
        train_x: &Vec<FeatureData>,
        train_y: &Vec<f64>,
        test_x: &Vec<FeatureData>,
        test_y: &Vec<f64>,
    ) -> MyResult<ForecastModel> {
        let matrix = DenseMatrix::from_2d_vec(&train_x);
        let r = KNNRegressor::fit(
            &matrix,
            &train_y,
            KNNRegressorParameters::default().with_distance(Distances::euclidian()),
        )?;
        let mut m = ForecastModel::KNN {
            pair: self.config.currency_pair.clone(),
            no: model_no,
            model: r,
            input_data_size: self.config.forecast_input_size,
            feature_params: params.clone(),
            performance_mse: Self::PERFORMANCE_MSE_DEFAULT,
            performance_rmse: Self::PERFORMANCE_RMSE_DEFAULT,
            memo: "KNN".to_string(),
        };

        m.update_performance(test_x, test_y)?;

        Ok(m)
    }

    fn make_linear(
        &self,
        model_no: i32,
        params: &FeatureParams,
        train_x: &Vec<FeatureData>,
        train_y: &Vec<f64>,
        test_x: &Vec<FeatureData>,
        test_y: &Vec<f64>,
    ) -> MyResult<ForecastModel> {
        let matrix = DenseMatrix::from_2d_vec(&train_x);
        let r = LinearRegression::fit(&matrix, &train_y, Default::default())?;
        let mut m = ForecastModel::Linear {
            pair: self.config.currency_pair.clone(),
            no: model_no,
            model: r,
            input_data_size: self.config.forecast_input_size,
            feature_params: params.clone(),
            performance_mse: Self::PERFORMANCE_MSE_DEFAULT,
            performance_rmse: Self::PERFORMANCE_RMSE_DEFAULT,
            memo: "Linear".to_string(),
        };

        m.update_performance(test_x, test_y)?;

        Ok(m)
    }

    fn make_ridge(
        &self,
        model_no: i32,
        params: &FeatureParams,
        train_x: &Vec<FeatureData>,
        train_y: &Vec<f64>,
        test_x: &Vec<FeatureData>,
        test_y: &Vec<f64>,
    ) -> MyResult<ForecastModel> {
        let matrix = DenseMatrix::from_2d_vec(&train_x);
        let r = RidgeRegression::fit(
            &matrix,
            &train_y,
            RidgeRegressionParameters::default().with_alpha(0.5),
        )?;
        let mut m = ForecastModel::Ridge {
            pair: self.config.currency_pair.clone(),
            no: model_no,
            model: r,
            input_data_size: self.config.forecast_input_size,
            feature_params: params.clone(),
            performance_mse: Self::PERFORMANCE_MSE_DEFAULT,
            performance_rmse: Self::PERFORMANCE_RMSE_DEFAULT,
            memo: "Ridge".to_string(),
        };

        m.update_performance(test_x, test_y)?;

        Ok(m)
    }

    fn make_lasso(
        &self,
        model_no: i32,
        params: &FeatureParams,
        train_x: &Vec<FeatureData>,
        train_y: &Vec<f64>,
        test_x: &Vec<FeatureData>,
        test_y: &Vec<f64>,
    ) -> MyResult<ForecastModel> {
        let matrix = DenseMatrix::from_2d_vec(&train_x);
        let r = Lasso::fit(
            &matrix,
            &train_y,
            LassoParameters::default().with_alpha(0.5),
        )?;
        let mut m = ForecastModel::LASSO {
            pair: self.config.currency_pair.clone(),
            no: model_no,
            model: r,
            input_data_size: self.config.forecast_input_size,
            feature_params: params.clone(),
            performance_mse: Self::PERFORMANCE_MSE_DEFAULT,
            performance_rmse: Self::PERFORMANCE_RMSE_DEFAULT,
            memo: "LASSO".to_string(),
        };

        m.update_performance(test_x, test_y)?;

        Ok(m)
    }

    fn make_elastic_net(
        &self,
        model_no: i32,
        params: &FeatureParams,
        train_x: &Vec<FeatureData>,
        train_y: &Vec<f64>,
        test_x: &Vec<FeatureData>,
        test_y: &Vec<f64>,
    ) -> MyResult<ForecastModel> {
        let matrix = DenseMatrix::from_2d_vec(&train_x);
        let r = ElasticNet::fit(
            &matrix,
            &train_y,
            ElasticNetParameters::default()
                .with_alpha(0.5)
                .with_l1_ratio(0.5),
        )?;
        let mut m = ForecastModel::ElasticNet {
            pair: self.config.currency_pair.clone(),
            no: model_no,
            model: r,
            input_data_size: self.config.forecast_input_size,
            feature_params: params.clone(),
            performance_mse: Self::PERFORMANCE_MSE_DEFAULT,
            performance_rmse: Self::PERFORMANCE_RMSE_DEFAULT,
            memo: "ElasticNet".to_string(),
        };

        m.update_performance(test_x, test_y)?;

        Ok(m)
    }

    fn make_svr(
        &self,
        model_no: i32,
        params: &FeatureParams,
        train_x: &Vec<FeatureData>,
        train_y: &Vec<f64>,
        test_x: &Vec<FeatureData>,
        test_y: &Vec<f64>,
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
        let mut m = ForecastModel::SVR {
            pair: self.config.currency_pair.clone(),
            no: model_no,
            model: r,
            input_data_size: self.config.forecast_input_size,
            feature_params: params.clone(),
            performance_mse: Self::PERFORMANCE_MSE_DEFAULT,
            performance_rmse: Self::PERFORMANCE_RMSE_DEFAULT,
            memo: "SVR".to_string(),
        };

        m.update_performance(test_x, test_y)?;

        Ok(m)
    }
}
