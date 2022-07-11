use chrono::{Duration, Utc};
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
    pub fn load(&self) -> MyResult<(Vec<InputData>, Vec<f64>)> {
        let end = Utc::now().naive_utc();
        let begin =
            (Utc::now() - Duration::hours(self.config.training_data_range_hour)).naive_utc();

        let (org_x, org_y) = util::load_input_data(self.config, self.mysql_cli, begin, end)?;
        let org_count = org_x.len();
        if org_count < self.config.training_data_required_count {
            return Err(Box::new(MyError::InputDataIsTooLittle {
                count: org_count,
                require: self.config.training_data_required_count,
            }));
        }

        Ok((org_x, org_y))
    }
}

pub struct ModelMaker<'a> {
    pub config: &'a config::Config,
    pub mysql_cli: &'a mysql::client::DefaultClient,
    pub train_base_x: &'a Vec<InputData>,
    pub train_base_y: &'a Vec<f64>,
    pub test_x: &'a Vec<InputData>,
    pub test_y: &'a Vec<f64>,
}

impl ModelMaker<'_> {
    const PERFORMANCE_MSE_DEFAULT: f64 = 1.0;

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

        let train_base_x = convert_to_features(self.train_base_x, params)?;
        let test_x = convert_to_features(self.test_x, params)?;

        for index in 1..=self.config.training_count {
            let (train_x, _, train_y, _) =
                util::train_test_split(&train_base_x, self.train_base_y, 0.2)?;

            debug!("training[{:2}] RandomForest ...", index);
            match self.make_random_forest(
                model_no,
                &params,
                &train_x,
                &train_y,
                &test_x,
                &self.test_y,
            ) {
                Ok(m) => {
                    models.push(m);
                }
                Err(err) => {
                    warn!(
                        "training[{:2}] skip RandomForest, error occured. error:{}",
                        index, err
                    );
                }
            }

            debug!("training[{:2}] KNN ...", index);
            match self.make_knn(model_no, &params, &train_x, &train_y, &test_x, &self.test_y) {
                Ok(m) => {
                    models.push(m);
                }
                Err(err) => {
                    warn!(
                        "training[{:2}] skip KNN, error occured. error:{}",
                        index, err
                    );
                }
            }

            debug!("training[{:2}] Linear ...", index);
            match self.make_linear(model_no, &params, &train_x, &train_y, &test_x, &self.test_y) {
                Ok(m) => {
                    models.push(m);
                }
                Err(err) => {
                    warn!(
                        "training[{:2}] skip Linear, error occured. error:{}",
                        index, err
                    );
                }
            }

            debug!("training[{:2}] Ridge ...", index);
            match self.make_ridge(model_no, &params, &train_x, &train_y, &test_x, &self.test_y) {
                Ok(m) => {
                    models.push(m);
                }
                Err(err) => {
                    warn!(
                        "training[{:2}] skip Ridge, error occured. error:{}",
                        index, err
                    );
                }
            }

            debug!("training[{:2}] LASSO ...", index);
            match self.make_lasso(model_no, &params, &train_x, &train_y, &test_x, &self.test_y) {
                Ok(m) => {
                    models.push(m);
                }
                Err(err) => {
                    warn!(
                        "training[{:2}] skip LASSO, error occured. error:{}",
                        index, err
                    );
                }
            }

            debug!("training[{:2}] ElasticNet ...", index);
            match self.make_elastic_net(
                model_no,
                &params,
                &train_x,
                &train_y,
                &test_x,
                &self.test_y,
            ) {
                Ok(m) => {
                    models.push(m);
                }
                Err(err) => {
                    warn!(
                        "training[{:2}] skip ElasticNet, error occured. error:{}",
                        index, err
                    );
                }
            }

            //  学習が終わらなかったためコメントアウト
            //  debug!("training[{:2}] Logistic ...", index);
            //  models.push(make_elastic_net(&p, &train_x, &train_y, config)?);

            debug!("training[{:2}] SVR ...", index);
            match self.make_svr(model_no, &params, &train_x, &train_y, &test_x, &self.test_y) {
                Ok(m) => {
                    models.push(m);
                }
                Err(err) => {
                    warn!(
                        "training[{:2}] skip SVR, error occured. error:{}",
                        index, err
                    );
                }
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
            memo: "ElasticNet".to_string(),
        };

        m.update_performance(test_x, test_y)?;

        Ok(m)
    }

    // fn make_ligistic(
    //     params: &FeatureParams,
    //     train_x: &Vec<FeatureData>,
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
    //         no: self.forecast_model_no,
    //         model: r,
    //         memo: "Logistic".to_string(),
    //     })
    // }

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
            memo: "SVR".to_string(),
        };

        m.update_performance(test_x, test_y)?;

        Ok(m)
    }
}
