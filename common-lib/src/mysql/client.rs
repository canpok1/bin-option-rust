use chrono::NaiveDateTime;
use mysql::{
    from_row, from_value, params, prelude::Queryable, Deserialized, OptsBuilder, Pool, Serialized,
    Transaction, TxOpts,
};

use crate::{
    domain::model::{ForecastModel, ForecastResult, RateForForecast, RateForTraining},
    error::MyResult,
    mysql::model::ForecastModelRecord,
};

static TABLE_NAME_RATE_FOR_TRAINING: &str = "rates_for_training";
static TABLE_NAME_FORECAST_MODEL: &str = "forecast_models";
static TABLE_NAME_RATE_FOR_FORECAST: &str = "rates_for_forecast";
static TABLE_NAME_FORECAST_RESULT: &str = "forecast_results";

pub trait Client {
    fn with_transaction<F>(&self, f: F) -> MyResult<()>
    where
        F: FnMut(&mut Transaction) -> MyResult<()>;

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

    fn insert_forecast_results(
        &self,
        tx: &mut Transaction,
        results: &Vec<ForecastResult>,
    ) -> MyResult<()>;
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
    //             // 任意のDB操作
    //             Ok(())
    //         }
    //     )
    // }
    // ```
    fn with_transaction<F>(&self, mut f: F) -> MyResult<()>
    where
        F: FnMut(&mut Transaction) -> MyResult<()>,
    {
        match self.pool.get_conn()?.start_transaction(TxOpts::default()) {
            Ok(mut tx) => match f(&mut tx) {
                Ok(_) => {
                    if let Err(err) = tx.commit() {
                        Err(Box::new(err))
                    } else {
                        Ok(())
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
            "INSERT INTO {} (pair, model_no, model_type, model_data, memo) VALUES (:pair, :no, :type, :data, :memo) ON DUPLICATE KEY UPDATE model_type = :type, model_data = :data, memo = :memo;",
            TABLE_NAME_FORECAST_MODEL
        );
        let p = match m {
            ForecastModel::RandomForest {
                pair,
                no,
                model: _,
                memo,
            } => {
                params! {
                    "pair" => pair,
                    "no" => no,
                    "type" => super::model::MODEL_TYPE_RANDOM_FOREST,
                    "data" => m.serialize_model_data()?,
                    "memo" => memo,
                }
            }
            ForecastModel::KNN {
                pair,
                no,
                model: _,
                memo,
            } => {
                params! {
                    "pair" => pair,
                    "no" => no,
                    "type" => super::model::MODEL_TYPE_KNN,
                    "data" => m.serialize_model_data()?,
                    "memo" => memo,
                }
            }
            ForecastModel::Linear {
                pair,
                no,
                model: _,
                memo,
            } => {
                params! {
                    "pair" => pair,
                    "no" => no,
                    "type" => super::model::MODEL_TYPE_LINEAR,
                    "data" => m.serialize_model_data()?,
                    "memo" => memo,
                }
            }
            ForecastModel::Ridge {
                pair,
                no,
                model: _,
                memo,
            } => {
                params! {
                    "pair" => pair,
                    "no" => no,
                    "type" => super::model::MODEL_TYPE_RIDGE,
                    "data" => m.serialize_model_data()?,
                    "memo" => memo,
                }
            }
            ForecastModel::LASSO {
                pair,
                no,
                model: _,
                memo,
            } => {
                params! {
                    "pair" => pair,
                    "no" => no,
                    "type" => super::model::MODEL_TYPE_LASSO,
                    "data" => m.serialize_model_data()?,
                    "memo" => memo,
                }
            }
            ForecastModel::ElasticNet {
                pair,
                no,
                model: _,
                memo,
            } => {
                params! {
                    "pair" => pair,
                    "no" => no,
                    "type" => super::model::MODEL_TYPE_ELASTIC_NET,
                    "data" => m.serialize_model_data()?,
                    "memo" => memo,
                }
            }
            ForecastModel::Logistic {
                pair,
                no,
                model: _,
                memo,
            } => {
                params! {
                    "pair" => pair,
                    "no" => no,
                    "type" => super::model::MODEL_TYPE_LOGISTIC,
                    "data" => m.serialize_model_data()?,
                    "memo" => memo,
                }
            }
            ForecastModel::SVR {
                pair,
                no,
                model: _,
                memo,
            } => {
                params! {
                    "pair" => pair,
                    "no" => no,
                    "type" => super::model::MODEL_TYPE_SVR,
                    "data" => m.serialize_model_data()?,
                    "memo" => memo,
                }
            }
        };
        log::debug!("query: {}, param: {}", q, m);

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
            "SELECT pair, model_no, model_type, model_data, memo, created_at, updated_at FROM {} WHERE pair = :pair AND model_no = :no",
            TABLE_NAME_FORECAST_MODEL
        );
        let p = params! {
            "pair" => pair,
            "no" => no,
        };
        log::debug!("query: {}, pair: {}, no: {}", q, pair, no);

        if let Some((pair, model_no, model_type, model_data, memo, created_at, updated_at)) =
            tx.exec_first(q, p)?
        {
            let record = ForecastModelRecord {
                pair,
                model_no,
                model_type,
                model_data,
                memo,
                created_at,
                updated_at,
            };
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
            "SELECT pair, model_no, model_type, model_data, memo, created_at, updated_at FROM {} WHERE pair = :pair",
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
                let (pair, model_no, model_type, model_data, memo, created_at, updated_at) =
                    from_row(row?);
                let record = ForecastModelRecord {
                    pair,
                    model_no,
                    model_type,
                    model_data,
                    memo,
                    created_at,
                    updated_at,
                };
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
}
