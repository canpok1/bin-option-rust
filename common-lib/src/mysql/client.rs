use chrono::NaiveDateTime;
use mysql::{
    from_row, from_value, params, prelude::Queryable, Deserialized, OptsBuilder, Pool, Serialized,
    Transaction, TxOpts,
};

use crate::{
    domain::model::{
        ForecastError, ForecastModel, ForecastResult, RateForForecast, RateForTraining,
        TrainingDataset,
    },
    error::MyResult,
    mysql::model::{FeatureParamsValue, ForecastModelRecord},
};

static TABLE_NAME_RATE_FOR_TRAINING: &str = "rates_for_training";
static TABLE_NAME_FORECAST_MODEL: &str = "forecast_models";
static TABLE_NAME_RATE_FOR_FORECAST: &str = "rates_for_forecast";
static TABLE_NAME_FORECAST_RESULT: &str = "forecast_results";
static TABLE_NAME_FORECAST_ERRORS: &str = "forecast_errors";
static TABLE_NAME_TRAINING_DATASETS: &str = "training_datasets";

pub trait Client {
    fn with_transaction<F, T>(&self, f: F) -> MyResult<T>
    where
        F: FnMut(&mut Transaction) -> MyResult<T>;

    fn insert_rates_for_training(
        &self,
        tx: &mut Transaction,
        rates: &Vec<RateForTraining>,
    ) -> MyResult<()>;
    fn delete_old_rates_for_training(
        &self,
        tx: &mut Transaction,
        border: &NaiveDateTime,
    ) -> MyResult<()>;
    fn select_rates_for_training(
        &self,
        tx: &mut Transaction,
        pair: &str,
        begin: Option<NaiveDateTime>,
        end: Option<NaiveDateTime>,
    ) -> MyResult<Vec<RateForTraining>>;

    fn upsert_forecast_model(&self, tx: &mut Transaction, m: &ForecastModel) -> MyResult<()>;
    fn copy_forecast_model(
        &self,
        tx: &mut Transaction,
        pair: &str,
        model_no_from: i32,
        model_no_to: i32,
    ) -> MyResult<()>;
    fn select_forecast_model(
        &self,
        tx: &mut Transaction,
        pair: &str,
        no: i32,
    ) -> MyResult<Option<ForecastModel>>;
    fn select_forecast_models(
        &self,
        tx: &mut Transaction,
        pair: &str,
    ) -> MyResult<Vec<ForecastModel>>;

    fn insert_rates_for_forecast(
        &self,
        tx: &mut Transaction,
        rate: &RateForForecast,
    ) -> MyResult<String>;
    fn select_rates_for_forecast_unforecasted(
        &self,
        tx: &mut Transaction,
        pair: &str,
    ) -> MyResult<Vec<RateForForecast>>;
    fn select_rates_for_forecast_by_id(
        &self,
        tx: &mut Transaction,
        id: &str,
    ) -> MyResult<Option<RateForForecast>>;
    fn delete_rates_for_forecast_expired(&self, tx: &mut Transaction) -> MyResult<()>;

    fn insert_forecast_results(
        &self,
        tx: &mut Transaction,
        results: &Vec<ForecastResult>,
    ) -> MyResult<()>;
    fn select_forecast_results_by_rate_id_and_model_no(
        &self,
        tx: &mut Transaction,
        rate_id: &str,
        model_no: i32,
    ) -> MyResult<Option<ForecastResult>>;
    fn delete_forecast_results_expired(&self, tx: &mut Transaction) -> MyResult<()>;

    fn insert_forecast_errors(
        &self,
        tx: &mut Transaction,
        records: &Vec<ForecastError>,
    ) -> MyResult<()>;
    fn select_forecast_errors_by_rate_id_and_model_no(
        &self,
        tx: &mut Transaction,
        rate_id: &str,
        model_no: i32,
    ) -> MyResult<Option<ForecastError>>;
    fn delete_forecast_errors_expired(&self, tx: &mut Transaction) -> MyResult<()>;

    fn insert_training_datasets(
        &self,
        tx: &mut Transaction,
        datasets: &Vec<TrainingDataset>,
    ) -> MyResult<()>;
    fn truncate_training_datasets(&self, tx: &mut Transaction) -> MyResult<()>;
}

#[derive(Clone, Debug)]
pub struct DefaultClient {
    pool: Pool,
}

impl DefaultClient {
    pub fn new(
        user: &str,
        password: &str,
        host: &str,
        port: u16,
        database: &str,
    ) -> MyResult<DefaultClient> {
        let opts = OptsBuilder::new()
            .user(Some(user))
            .pass(Some(password))
            .ip_or_hostname(Some(host))
            .tcp_port(port)
            .db_name(Some(database));

        Ok(DefaultClient {
            pool: Pool::new(opts)?,
        })
    }
}

impl Client for DefaultClient {
    // sample
    // ```
    // use crate::common_lib::error::MyResult;
    // use crate::common_lib::mysql::client::DefaultClient;
    // use crate::common_lib::mysql::client::Client;
    //
    // fn main() -> MyResult<()> {
    //     let client = DefaultClient::new("user", "pass", "127.0.0.1", 3306, "db")?;
    //     client.with_transaction(
    //         |tx| -> MyResult<()> {
    //             // ?????????DB??????
    //             Ok(())
    //         }
    //     )
    // }
    // ```
    fn with_transaction<F, T>(&self, mut f: F) -> MyResult<T>
    where
        F: FnMut(&mut Transaction) -> MyResult<T>,
    {
        match self.pool.get_conn()?.start_transaction(TxOpts::default()) {
            Ok(mut tx) => match f(&mut tx) {
                Ok(v) => {
                    if let Err(err) = tx.commit() {
                        Err(Box::new(err))
                    } else {
                        Ok(v)
                    }
                }
                Err(err) => Err(err),
            },
            Err(err) => Err(Box::new(err)),
        }
    }

    fn insert_rates_for_training(
        &self,
        tx: &mut Transaction,
        rates: &Vec<RateForTraining>,
    ) -> MyResult<()> {
        tx.exec_batch(
            format!(
                "INSERT INTO {} (pair, recorded_at, rate) VALUES (:pair, :recorded_at, :rate);",
                TABLE_NAME_RATE_FOR_TRAINING
            ),
            rates.iter().map(|rate| {
                params! {
                    "pair" => &rate.pair,
                    "recorded_at" => rate.recorded_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                    "rate" => &rate.rate,
                }
            }),
        )?;

        Ok(())
    }

    fn delete_old_rates_for_training(
        &self,
        tx: &mut Transaction,
        border: &NaiveDateTime,
    ) -> MyResult<()> {
        tx.exec_drop(
            format!(
                "DELETE FROM {} WHERE recorded_at < :border;",
                TABLE_NAME_RATE_FOR_TRAINING
            ),
            params! {
                "border" => border.format("%Y-%m-%d %H:%M:%S").to_string(),
            },
        )?;

        Ok(())
    }

    fn select_rates_for_training(
        &self,
        tx: &mut Transaction,
        pair: &str,
        begin: Option<NaiveDateTime>,
        end: Option<NaiveDateTime>,
    ) -> MyResult<Vec<RateForTraining>> {
        let mut conditions: Vec<String> = vec![];
        if let Some(value) = begin {
            conditions.push(format!(
                "recorded_at >= '{}'",
                value.format("%Y-%m-%d %H:%M:%S")
            ));
        }
        if let Some(value) = end {
            conditions.push(format!(
                "recorded_at <= '{}'",
                value.format("%Y-%m-%d %H:%M:%S")
            ));
        }
        let mut where_str = format!("WHERE pair = '{}'", pair);
        if !conditions.is_empty() {
            where_str = format!("{} AND {}", where_str, conditions.join(" AND "));
        };

        let query = format!(
            "SELECT pair, recorded_at, rate, created_at, updated_at FROM {} {} ORDER BY recorded_at ASC",
            TABLE_NAME_RATE_FOR_TRAINING,
            where_str,
        );
        log::debug!("query: {}", query);

        let result = tx.query_map(
            query,
            |(pair, recorded_at, rate, created_at, updated_at)| RateForTraining {
                pair,
                recorded_at,
                rate,
                created_at,
                updated_at,
            },
        );
        Ok(result?)
    }

    fn upsert_forecast_model(&self, tx: &mut Transaction, m: &ForecastModel) -> MyResult<()> {
        let q = format!(
            r#"
                INSERT INTO {}
                    (pair, model_no, model_type, model_data, input_data_size, feature_params, feature_params_hash, performance_mse, performance_rmse, memo)
                VALUES
                    (:pair, :no, :type, :data, :input_data_size, :feature_params, :feature_params_hash, :performance_mse, :performance_rmse, :memo)
                ON DUPLICATE KEY UPDATE
                    model_type = :type,
                    model_data = :data,
                    input_data_size = :input_data_size,
                    feature_params = :feature_params,
                    feature_params_hash = :feature_params_hash,
                    performance_mse = :performance_mse,
                    performance_rmse = :performance_rmse,
                    memo = :memo;
            "#,
            TABLE_NAME_FORECAST_MODEL
        );
        let p = match m {
            ForecastModel::RandomForest {
                pair,
                no,
                input_data_size,
                feature_params,
                performance_mse,
                performance_rmse,
                memo,
                ..
            } => {
                params! {
                    "pair" => pair,
                    "no" => no,
                    "type" => super::model::MODEL_TYPE_RANDOM_FOREST,
                    "data" => m.serialize_model_data()?,
                    "input_data_size" => input_data_size,
                    "feature_params" => Serialized(feature_params),
                    "feature_params_hash" => feature_params.to_hash()?,
                    "performance_mse" => performance_mse,
                    "performance_rmse" => performance_rmse,
                    "memo" => memo,
                }
            }
            ForecastModel::KNN {
                pair,
                no,
                input_data_size,
                feature_params,
                performance_mse,
                performance_rmse,
                memo,
                ..
            } => {
                params! {
                    "pair" => pair,
                    "no" => no,
                    "type" => super::model::MODEL_TYPE_KNN,
                    "data" => m.serialize_model_data()?,
                    "input_data_size" => input_data_size,
                    "feature_params" => Serialized(feature_params),
                    "feature_params_hash" => feature_params.to_hash()?,
                    "performance_mse" => performance_mse,
                    "performance_rmse" => performance_rmse,
                    "memo" => memo,
                }
            }
            ForecastModel::Linear {
                pair,
                no,
                input_data_size,
                feature_params,
                performance_mse,
                performance_rmse,
                memo,
                ..
            } => {
                params! {
                    "pair" => pair,
                    "no" => no,
                    "type" => super::model::MODEL_TYPE_LINEAR,
                    "data" => m.serialize_model_data()?,
                    "input_data_size" => input_data_size,
                    "feature_params" => Serialized(feature_params),
                    "feature_params_hash" => feature_params.to_hash()?,
                    "performance_mse" => performance_mse,
                    "performance_rmse" => performance_rmse,
                    "memo" => memo,
                }
            }
            ForecastModel::Ridge {
                pair,
                no,
                input_data_size,
                feature_params,
                performance_mse,
                performance_rmse,
                memo,
                ..
            } => {
                params! {
                    "pair" => pair,
                    "no" => no,
                    "type" => super::model::MODEL_TYPE_RIDGE,
                    "data" => m.serialize_model_data()?,
                    "input_data_size" => input_data_size,
                    "feature_params" => Serialized(feature_params),
                    "feature_params_hash" => feature_params.to_hash()?,
                    "performance_mse" => performance_mse,
                    "performance_rmse" => performance_rmse,
                    "memo" => memo,
                }
            }
            ForecastModel::LASSO {
                pair,
                no,
                input_data_size,
                feature_params,
                performance_mse,
                performance_rmse,
                memo,
                ..
            } => {
                params! {
                    "pair" => pair,
                    "no" => no,
                    "type" => super::model::MODEL_TYPE_LASSO,
                    "data" => m.serialize_model_data()?,
                    "input_data_size" => input_data_size,
                    "feature_params" => Serialized(feature_params),
                    "feature_params_hash" => feature_params.to_hash()?,
                    "performance_mse" => performance_mse,
                    "performance_rmse" => performance_rmse,
                    "memo" => memo,
                }
            }
            ForecastModel::ElasticNet {
                pair,
                no,
                input_data_size,
                feature_params,
                performance_mse,
                performance_rmse,
                memo,
                ..
            } => {
                params! {
                    "pair" => pair,
                    "no" => no,
                    "type" => super::model::MODEL_TYPE_ELASTIC_NET,
                    "data" => m.serialize_model_data()?,
                    "input_data_size" => input_data_size,
                    "feature_params" => Serialized(feature_params),
                    "feature_params_hash" => feature_params.to_hash()?,
                    "performance_mse" => performance_mse,
                    "performance_rmse" => performance_rmse,
                    "memo" => memo,
                }
            }
            ForecastModel::Logistic {
                pair,
                no,
                input_data_size,
                feature_params,
                performance_mse,
                performance_rmse,
                memo,
                ..
            } => {
                params! {
                    "pair" => pair,
                    "no" => no,
                    "type" => super::model::MODEL_TYPE_LOGISTIC,
                    "data" => m.serialize_model_data()?,
                    "input_data_size" => input_data_size,
                    "feature_params" => Serialized(feature_params),
                    "feature_params_hash" => feature_params.to_hash()?,
                    "performance_mse" => performance_mse,
                    "performance_rmse" => performance_rmse,
                    "memo" => memo,
                }
            }
            ForecastModel::SVR {
                pair,
                no,
                input_data_size,
                feature_params,
                performance_mse,
                performance_rmse,
                memo,
                ..
            } => {
                params! {
                    "pair" => pair,
                    "no" => no,
                    "type" => super::model::MODEL_TYPE_SVR,
                    "data" => m.serialize_model_data()?,
                    "input_data_size" => input_data_size,
                    "feature_params" => Serialized(feature_params),
                    "feature_params_hash" => feature_params.to_hash()?,
                    "performance_mse" => performance_mse,
                    "performance_rmse" => performance_rmse,
                    "memo" => memo,
                }
            }
        };
        log::debug!("query: {}, param: {}", q, m);

        tx.exec_drop(q, p)?;

        Ok(())
    }

    fn copy_forecast_model(
        &self,
        tx: &mut Transaction,
        pair: &str,
        model_no_from: i32,
        model_no_to: i32,
    ) -> MyResult<()> {
        let q = format!(
            r#"
                INSERT INTO {0}
                    (pair, model_no, model_type, model_data, input_data_size, feature_params, feature_params_hash, performance_mse, performance_rmse, memo)
                SELECT
                    pair, model_no, model_type, model_data, input_data_size, feature_params, feature_params_hash, performance_mse, performance_rmse, memo
                FROM (
                    SELECT
                        pair, :model_no_to model_no, model_type, model_data, input_data_size, feature_params, feature_params_hash, performance_mse, performance_rmse, memo
                    FROM {0}
                    WHERE pair = :pair AND model_no = :model_no_from
                ) t
                ON DUPLICATE KEY UPDATE
                    model_type = t.model_type,
                    model_data = t.model_data,
                    input_data_size = t.input_data_size,
                    feature_params = t.feature_params,
                    feature_params_hash = t.feature_params_hash,
                    performance_mse = t.performance_mse,
                    performance_rmse = t.performance_rmse,
                    memo = t.memo;
            "#,
            TABLE_NAME_FORECAST_MODEL
        );
        let p = params! {
            "pair" => pair,
            "model_no_from" => model_no_from,
            "model_no_to" => model_no_to,
        };

        log::debug!("query: {}, {:?}", q, p);

        tx.exec_drop(q, p)?;

        Ok(())
    }

    fn select_forecast_model(
        &self,
        tx: &mut Transaction,
        pair: &str,
        no: i32,
    ) -> MyResult<Option<ForecastModel>> {
        let q = format!(
            r#"
                SELECT
                    pair, model_no, model_type, model_data, input_data_size, feature_params, feature_params_hash, performance_mse, performance_rmse, memo, created_at, updated_at
                FROM {}
                WHERE
                    pair = :pair AND model_no = :no;
            "#,
            TABLE_NAME_FORECAST_MODEL
        );
        let p = params! {
            "pair" => pair,
            "no" => no,
        };
        log::debug!("query: {}, pair: {}, no: {}", q, pair, no);

        if let Some((
            pair,
            model_no,
            model_type,
            model_data,
            input_data_size,
            feature_params_raw,
            feature_params_hash,
            performance_mse,
            performance_rmse,
            memo,
            created_at,
            updated_at,
        )) = tx.exec_first(q, p)?
        {
            let Deserialized(feature_params_value): Deserialized<FeatureParamsValue> =
                from_value(feature_params_raw);
            let record = ForecastModelRecord {
                pair,
                model_no,
                model_type,
                model_data,
                input_data_size,
                feature_params: feature_params_value.to_domain()?,
                feature_params_hash,
                performance_mse,
                performance_rmse,
                memo,
                created_at,
                updated_at,
            };
            if let Err(err) = record.validate_feature_params() {
                log::warn!("model not found, {}", err);
                return Ok(None);
            }
            Ok(Some(record.to_domain()?))
        } else {
            Ok(None)
        }
    }

    fn select_forecast_models(
        &self,
        tx: &mut Transaction,
        pair: &str,
    ) -> MyResult<Vec<ForecastModel>> {
        let q = format!(
            r#"
                SELECT
                    pair, model_no, model_type, model_data, input_data_size, feature_params, feature_params_hash, performance_mse, performance_rmse, memo, created_at, updated_at
                FROM {}
                WHERE
                    pair = :pair
            "#,
            TABLE_NAME_FORECAST_MODEL
        );
        let p = params! {
            "pair" => pair,
        };
        log::debug!("query: {}, pair: {}", q, pair);

        let mut models: Vec<ForecastModel> = vec![];
        let mut result = tx.exec_iter(q, p)?;
        while let Some(result_set) = result.next_set() {
            for row in result_set? {
                let (
                    pair,
                    model_no,
                    model_type,
                    model_data,
                    input_data_size,
                    feature_params_raw,
                    feature_params_hash,
                    performance_mse,
                    performance_rmse,
                    memo,
                    created_at,
                    updated_at,
                ) = from_row(row?);
                let Deserialized(feature_params_value): Deserialized<FeatureParamsValue> =
                    from_value(feature_params_raw);
                let record = ForecastModelRecord {
                    pair,
                    model_no,
                    model_type,
                    model_data,
                    input_data_size,
                    feature_params: feature_params_value.to_domain()?,
                    feature_params_hash,
                    performance_mse,
                    performance_rmse,
                    memo,
                    created_at,
                    updated_at,
                };
                if let Err(err) = record.validate_feature_params() {
                    log::warn!("model not found, {}", err);
                    continue;
                }
                models.push(record.to_domain()?);
            }
        }
        Ok(models)
    }

    fn insert_rates_for_forecast(
        &self,
        tx: &mut Transaction,
        rate: &RateForForecast,
    ) -> MyResult<String> {
        let id: Option<String> = tx.query_first("SELECT UUID();")?;
        tx.exec_drop(
            format!(
                "INSERT INTO {} (id, pair, histories, expire, memo) VALUES (:id, :pair, :histories, :expire, :memo);",
                TABLE_NAME_RATE_FOR_FORECAST
            ),
            params! {
                "id" => &id,
                "pair" => &rate.pair,
                "histories" => Serialized(&rate.histories),
                "expire" => &rate.expire,
                "memo" => &rate.memo,
            },
        )?;
        Ok(id.unwrap())
    }

    fn select_rates_for_forecast_unforecasted(
        &self,
        tx: &mut Transaction,
        pair: &str,
    ) -> MyResult<Vec<RateForForecast>> {
        let q = format!(
            r#"
                WITH forecasted AS (
                    SELECT DISTINCT rate_id FROM {}
                )
                SELECT f.id, f.pair, f.histories, f.expire, f.memo, f.created_at, f.updated_at
                FROM {} f
                LEFT OUTER JOIN forecasted ON f.id = forecasted.rate_id
                WHERE
                    f.pair = :pair AND forecasted.rate_id IS NULL
            "#,
            TABLE_NAME_FORECAST_RESULT, TABLE_NAME_RATE_FOR_FORECAST,
        );
        let p = params! {
            "pair" => pair,
        };
        log::debug!("query: {}, pair: {}", q, pair);

        let mut rates: Vec<RateForForecast> = vec![];
        let mut result = tx.exec_iter(q, p)?;
        while let Some(result_set) = result.next_set() {
            for row in result_set? {
                let (id, pair, histories_raw, expire, memo, created_at, updated_at) =
                    from_row(row?);
                let Deserialized(histories): Deserialized<Vec<f64>> = from_value(histories_raw);
                let record = RateForForecast {
                    id,
                    pair,
                    histories,
                    expire,
                    memo,
                    created_at,
                    updated_at,
                };
                rates.push(record);
            }
        }
        Ok(rates)
    }

    fn select_rates_for_forecast_by_id(
        &self,
        tx: &mut Transaction,
        id: &str,
    ) -> MyResult<Option<RateForForecast>> {
        let q = format!(
            r#"
                SELECT id, pair, histories, expire, memo, created_at, updated_at
                FROM {}
                WHERE id = :id AND expire >= CURRENT_TIMESTAMP();
            "#,
            TABLE_NAME_RATE_FOR_FORECAST,
        );
        let p = params! {
            "id" => id,
        };
        log::debug!("query: {}, id: {}", q, id);

        if let Some((id, pair, histories_raw, expire, memo, created_at, updated_at)) =
            tx.exec_first(q, p)?
        {
            let Deserialized(histories) = from_value(histories_raw);
            let record = RateForForecast {
                id,
                pair,
                histories: histories,
                expire,
                memo,
                created_at,
                updated_at,
            };
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    fn delete_rates_for_forecast_expired(&self, tx: &mut Transaction) -> MyResult<()> {
        let q = format!(
            "DELETE FROM {} WHERE expire < CURRENT_TIMESTAMP();",
            TABLE_NAME_RATE_FOR_FORECAST
        );
        tx.query_drop(q)?;

        Ok(())
    }

    fn insert_forecast_results(
        &self,
        tx: &mut Transaction,
        results: &Vec<ForecastResult>,
    ) -> MyResult<()> {
        tx.exec_batch(
            format!(
                "INSERT INTO {} (rate_id, model_no, forecast_type, result, memo) VALUES (:rate_id, :model_no, :forecast_type, :result, :memo);",
                TABLE_NAME_FORECAST_RESULT,
            ),
            results.iter().map(|result| {
                params! {
                    "rate_id" => &result.rate_id,
                    "model_no" => &result.model_no,
                    "forecast_type" => &result.forecast_type,
                    "result" => &result.result,
                    "memo" => &result.memo,
                }
            }),
        )?;

        Ok(())
    }

    fn select_forecast_results_by_rate_id_and_model_no(
        &self,
        tx: &mut Transaction,
        rate_id: &str,
        model_no: i32,
    ) -> MyResult<Option<ForecastResult>> {
        let q = format!(
            r#"
                SELECT id, rate_id, model_no, forecast_type, result, memo, created_at, updated_at
                FROM {}
                WHERE rate_id = :rate_id AND model_no = :model_no;
            "#,
            TABLE_NAME_FORECAST_RESULT,
        );
        let p = params! {
            "rate_id" => rate_id,
            "model_no" => model_no,
        };
        log::debug!("query: {}, rate_id: {}, model_no: {}", q, rate_id, model_no);

        if let Some((id, rate_id, model_no, forecast_type, result, memo, created_at, updated_at)) =
            tx.exec_first(q, p)?
        {
            let record = ForecastResult {
                id,
                rate_id,
                model_no,
                forecast_type,
                result,
                memo,
                created_at,
                updated_at,
            };
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    fn delete_forecast_results_expired(&self, tx: &mut Transaction) -> MyResult<()> {
        let q = format!(
            r#"
                DELETE FROM {} WHERE rate_id IN (
                    SELECT id FROM {} WHERE expire < CURRENT_TIMESTAMP()
                );
            "#,
            TABLE_NAME_FORECAST_RESULT, TABLE_NAME_RATE_FOR_FORECAST
        );
        tx.query_drop(q)?;

        Ok(())
    }

    fn insert_forecast_errors(
        &self,
        tx: &mut Transaction,
        records: &Vec<ForecastError>,
    ) -> MyResult<()> {
        tx.exec_batch(
            format!(
                "INSERT INTO {} (rate_id, model_no, summary, detail) VALUES (:rate_id, :model_no, :summary, :detail);",
                TABLE_NAME_FORECAST_ERRORS,
            ),
            records.iter().map(|record| {
                params! {
                    "rate_id" => &record.rate_id,
                    "model_no" => &record.model_no,
                    "summary" => &record.summary,
                    "detail" => &record.detail,
                }
            }),
        )?;

        Ok(())
    }

    fn select_forecast_errors_by_rate_id_and_model_no(
        &self,
        tx: &mut Transaction,
        rate_id: &str,
        model_no: i32,
    ) -> MyResult<Option<ForecastError>> {
        let q = format!(
            r#"
                SELECT id, rate_id, model_no, summary, detail
                FROM {}
                WHERE rate_id = :rate_id AND model_no = :model_no;
            "#,
            TABLE_NAME_FORECAST_ERRORS,
        );
        let p = params! {
            "rate_id" => rate_id,
            "model_no" => model_no,
        };
        log::debug!("query: {}, rate_id: {}, model_no: {}", q, rate_id, model_no);

        if let Some((id, rate_id, model_no, summary, detail)) = tx.exec_first(q, p)? {
            let record = ForecastError {
                id,
                rate_id,
                model_no,
                summary,
                detail,
            };
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    fn delete_forecast_errors_expired(&self, tx: &mut Transaction) -> MyResult<()> {
        let q = format!(
            r#"
                DELETE FROM {} WHERE rate_id IN (
                    SELECT id FROM {} WHERE expire < CURRENT_TIMESTAMP()
                );
            "#,
            TABLE_NAME_FORECAST_ERRORS, TABLE_NAME_RATE_FOR_FORECAST
        );
        tx.query_drop(q)?;

        Ok(())
    }

    fn insert_training_datasets(
        &self,
        tx: &mut Transaction,
        datasets: &Vec<TrainingDataset>,
    ) -> MyResult<()> {
        tx.exec_batch(
            format!(
                "INSERT INTO {} (pair, input_data, truth, memo) VALUES (:pair, :input_data, :truth, :memo);",
                TABLE_NAME_TRAINING_DATASETS
            ),
            datasets.iter().map(|dataset| {
                params! {
                    "pair" => &dataset.pair,
                    "input_data" => Serialized(&dataset.input_data),
                    "truth" => &dataset.truth,
                    "memo" => &dataset.memo,
                }
            }),
        )?;

        Ok(())
    }

    fn truncate_training_datasets(&self, tx: &mut Transaction) -> MyResult<()> {
        let q = format!(" TRUNCATE TABLE {};", TABLE_NAME_TRAINING_DATASETS);
        tx.query_drop(q)?;

        Ok(())
    }
}
