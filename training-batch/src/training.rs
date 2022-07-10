use common_lib::{
    domain::model::{FeatureParams, ForecastModel},
    error::MyResult,
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
    metrics::mean_squared_error,
    neighbors::knn_regressor::{KNNRegressor, KNNRegressorParameters},
    svm::{
        svr::{SVRParameters, SVR},
        Kernels,
    },
};

use crate::{config, util};

pub struct Trainer<'a> {
    pub config: &'a config::Config,
    pub mysql_cli: &'a mysql::client::DefaultClient,
}

impl Trainer<'_> {
    const PERFORMANCE_MSE_DEFAULT: f64 = 1.0;

    pub fn training(
        &self,
        p: &FeatureParams,
        train_base_x: &Vec<Vec<f64>>,
        train_base_y: &Vec<f64>,
        test_x: &Vec<Vec<f64>>,
        test_y: &Vec<f64>,
    ) -> MyResult<()> {
        let mut models: Vec<ForecastModel> = vec![];

        if let Some(m) = self.load_existing_model()? {
            let input_data_size = m.get_input_data_size()?;
            if input_data_size == self.config.forecast_input_size {
                models.push(m);
            } else {
                warn!(
                    "input data size is not match, not use existing model. model: {}, training: {}",
                    input_data_size, self.config.forecast_input_size
                );
            }
        }

        models.extend(self.make_new_models(p, train_base_x, train_base_y)?);

        if let Some((index, mse)) = Self::find_best_model_index(&models, &test_x, &test_y)? {
            if let Some(model) = models.get_mut(index) {
                model.set_performance_mse(mse)?;
                self.save_model(&model)?;
            }
        }

        Ok(())
    }

    fn load_existing_model(&self) -> MyResult<Option<ForecastModel>> {
        let mut model: Option<ForecastModel> = None;
        self.mysql_cli.with_transaction(|tx| -> MyResult<()> {
            model = self.mysql_cli.select_forecast_model(
                tx,
                &self.config.currency_pair,
                self.config.forecast_model_no,
            )?;
            Ok(())
        })?;
        Ok(model)
    }

    fn make_new_models(
        &self,
        p: &FeatureParams,
        train_base_x: &Vec<Vec<f64>>,
        train_base_y: &Vec<f64>,
    ) -> MyResult<Vec<ForecastModel>> {
        let mut models: Vec<ForecastModel> = vec![];

        for index in 1..=self.config.training_count {
            let (train_x, _, train_y, _) = util::train_test_split(train_base_x, train_base_y, 0.2)?;

            debug!("training[{:2}] RandomForest ...", index);
            models.push(self.make_random_forest(&p, &train_x, &train_y)?);

            debug!("training[{:2}] KNN ...", index);
            models.push(self.make_knn(&p, &train_x, &train_y)?);

            debug!("training[{:2}] Linear ...", index);
            models.push(self.make_linear(&p, &train_x, &train_y)?);

            debug!("training[{:2}] Ridge ...", index);
            models.push(self.make_ridge(&p, &train_x, &train_y)?);

            debug!("training[{:2}] LASSO ...", index);
            models.push(self.make_lasso(&p, &train_x, &train_y)?);

            debug!("training[{:2}] ElasticNet ...", index);
            models.push(self.make_elastic_net(&p, &train_x, &train_y)?);

            //  学習が終わらなかったためコメントアウト
            //  debug!("training[{:2}] Logistic ...", index);
            //  models.push(make_elastic_net(&p, &train_x, &train_y, config)?);

            debug!("training[{:2}] SVR ...", index);
            models.push(self.make_svr(&p, &train_x, &train_y)?);
        }

        Ok(models)
    }

    fn make_random_forest(
        &self,
        params: &FeatureParams,
        train_x: &Vec<Vec<f64>>,
        train_y: &Vec<f64>,
    ) -> MyResult<ForecastModel> {
        let matrix = DenseMatrix::from_2d_vec(&train_x);
        Ok(ForecastModel::RandomForest {
            pair: self.config.currency_pair.clone(),
            no: self.config.forecast_model_no,
            model: RandomForestRegressor::fit(&matrix, &train_y, Default::default())?,
            input_data_size: self.config.forecast_input_size,
            feature_params: params.clone(),
            performance_mse: Self::PERFORMANCE_MSE_DEFAULT,
            memo: "RandomForest".to_string(),
        })
    }

    fn make_knn(
        &self,
        params: &FeatureParams,
        train_x: &Vec<Vec<f64>>,
        train_y: &Vec<f64>,
    ) -> MyResult<ForecastModel> {
        let matrix = DenseMatrix::from_2d_vec(&train_x);
        let r = KNNRegressor::fit(
            &matrix,
            &train_y,
            KNNRegressorParameters::default().with_distance(Distances::euclidian()),
        )?;
        Ok(ForecastModel::KNN {
            pair: self.config.currency_pair.clone(),
            no: self.config.forecast_model_no,
            model: r,
            input_data_size: self.config.forecast_input_size,
            feature_params: params.clone(),
            performance_mse: Self::PERFORMANCE_MSE_DEFAULT,
            memo: "KNN".to_string(),
        })
    }

    fn make_linear(
        &self,
        params: &FeatureParams,
        train_x: &Vec<Vec<f64>>,
        train_y: &Vec<f64>,
    ) -> MyResult<ForecastModel> {
        let matrix = DenseMatrix::from_2d_vec(&train_x);
        let r = LinearRegression::fit(&matrix, &train_y, Default::default())?;
        Ok(ForecastModel::Linear {
            pair: self.config.currency_pair.clone(),
            no: self.config.forecast_model_no,
            model: r,
            input_data_size: self.config.forecast_input_size,
            feature_params: params.clone(),
            performance_mse: Self::PERFORMANCE_MSE_DEFAULT,
            memo: "Linear".to_string(),
        })
    }

    fn make_ridge(
        &self,
        params: &FeatureParams,
        train_x: &Vec<Vec<f64>>,
        train_y: &Vec<f64>,
    ) -> MyResult<ForecastModel> {
        let matrix = DenseMatrix::from_2d_vec(&train_x);
        let r = RidgeRegression::fit(
            &matrix,
            &train_y,
            RidgeRegressionParameters::default().with_alpha(0.5),
        )?;
        Ok(ForecastModel::Ridge {
            pair: self.config.currency_pair.clone(),
            no: self.config.forecast_model_no,
            model: r,
            input_data_size: self.config.forecast_input_size,
            feature_params: params.clone(),
            performance_mse: Self::PERFORMANCE_MSE_DEFAULT,
            memo: "Ridge".to_string(),
        })
    }

    fn make_lasso(
        &self,
        params: &FeatureParams,
        train_x: &Vec<Vec<f64>>,
        train_y: &Vec<f64>,
    ) -> MyResult<ForecastModel> {
        let matrix = DenseMatrix::from_2d_vec(&train_x);
        let r = Lasso::fit(
            &matrix,
            &train_y,
            LassoParameters::default().with_alpha(0.5),
        )?;
        Ok(ForecastModel::LASSO {
            pair: self.config.currency_pair.clone(),
            no: self.config.forecast_model_no,
            model: r,
            input_data_size: self.config.forecast_input_size,
            feature_params: params.clone(),
            performance_mse: Self::PERFORMANCE_MSE_DEFAULT,
            memo: "LASSO".to_string(),
        })
    }

    fn make_elastic_net(
        &self,
        params: &FeatureParams,
        train_x: &Vec<Vec<f64>>,
        train_y: &Vec<f64>,
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
            pair: self.config.currency_pair.clone(),
            no: self.config.forecast_model_no,
            model: r,
            input_data_size: self.config.forecast_input_size,
            feature_params: params.clone(),
            performance_mse: Self::PERFORMANCE_MSE_DEFAULT,
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
        &self,
        params: &FeatureParams,
        train_x: &Vec<Vec<f64>>,
        train_y: &Vec<f64>,
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
            pair: self.config.currency_pair.clone(),
            no: self.config.forecast_model_no,
            model: r,
            input_data_size: self.config.forecast_input_size,
            feature_params: params.clone(),
            performance_mse: Self::PERFORMANCE_MSE_DEFAULT,
            memo: "SVR".to_string(),
        })
    }

    fn find_best_model_index(
        models: &Vec<ForecastModel>,
        test_x: &Vec<Vec<f64>>,
        test_y: &Vec<f64>,
    ) -> MyResult<Option<(usize, f64)>> {
        let matrix = DenseMatrix::from_2d_vec(test_x);

        let mut best_model_index: Option<usize> = None;
        let mut best_mse: Option<f64> = None;
        for (i, m) in models.iter().enumerate() {
            let y = m.predict_for_training(&matrix)?;

            let mse = mean_squared_error(test_y, &y);
            if best_mse.is_none() || mse < best_mse.unwrap() {
                best_model_index = Some(i);
                best_mse = Some(mse);
            }
        }

        if let Some(mse) = best_mse {
            Ok(Some((best_model_index.unwrap(), mse)))
        } else {
            Ok(None)
        }
    }

    fn save_model(&self, model: &ForecastModel) -> MyResult<()> {
        self.mysql_cli.with_transaction(|tx| {
            self.mysql_cli.upsert_forecast_model(tx, model)?;
            Ok(())
        })?;
        Ok(())
    }
}
