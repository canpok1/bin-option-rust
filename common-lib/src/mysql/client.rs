use chrono::NaiveDateTime;
use mysql::{params, prelude::Queryable, OptsBuilder, Pool, TxOpts, Transaction};

use crate::{
    error::MyResult,
    domain::model::RateForTraining
};

use super::model::ForecastModel;

static TABLE_NAME_RATE_FOR_TRAINING:&str = "rates_for_training";
static TABLE_NAME_FORECAST_MODEL:&str = "forecast_models";

pub trait Client
{
    fn with_transaction<F>(&self, f: F) -> MyResult<()>
    where
        F: FnMut(&mut Transaction) -> MyResult<()>
    ;

    fn insert_rates_for_training(&self, tx: &mut Transaction, rates: &Vec<RateForTraining>) -> MyResult<()>;
    fn delete_old_rates_for_training(&self, tx: &mut Transaction, border: &NaiveDateTime) -> MyResult<()>;
    fn select_rates_for_training(&self, tx: &mut Transaction, pair: &str, begin: Option<NaiveDateTime>, end: Option<NaiveDateTime>) -> MyResult<Vec<RateForTraining>>;

    fn upsert_forecast_model(&self, tx: &mut Transaction, m: &ForecastModel) -> MyResult<()>;
    fn select_forecast_model(&self, tx: &mut Transaction, pair: &str, no:i32) -> MyResult<Option<ForecastModel>>;
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

impl Client for DefaultClient
{
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
        F: FnMut(&mut Transaction) -> MyResult<()>
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
                Err(err) => {
                    Err(err)
                }
            },
            Err(err) => Err(Box::new(err)),
        }
    }

    fn insert_rates_for_training(&self, tx: &mut Transaction, rates: &Vec<RateForTraining>) -> MyResult<()> {
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

    fn delete_old_rates_for_training(&self, tx: &mut Transaction, border: &NaiveDateTime) -> MyResult<()> {
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

    fn select_rates_for_training(&self, tx: &mut Transaction, pair: &str, begin: Option<NaiveDateTime>, end: Option<NaiveDateTime>) -> MyResult<Vec<RateForTraining>> {
        let mut conditions:Vec<String> = vec![];
        if let Some(value) = begin {
            conditions.push(format!("recorded_at >= '{}'", value.format("%Y-%m-%d %H:%M:%S")));
        }
        if let Some(value) = end {
            conditions.push(format!("recorded_at <= '{}'", value.format("%Y-%m-%d %H:%M:%S")));
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
            |(pair, recorded_at, rate, created_at, updated_at)| {
                RateForTraining {
                    pair,
                    recorded_at,
                    rate,
                    created_at,
                    updated_at,
                }
            },
        );
        Ok(result?)
    }


    fn upsert_forecast_model(&self, tx: &mut Transaction, m: &ForecastModel) -> MyResult<()> {
        let q = format!(
            "INSERT INTO {} (pair, model_no, model_data, memo) VALUES (:pair, :no, :data, :memo) ON DUPLICATE KEY UPDATE model_data = :data, memo = :memo;",
            TABLE_NAME_FORECAST_MODEL
        );
        let p = params! {
            "pair" => &m.pair,
            "no" => m.no,
            "data" => &m.data,
            "memo" => &m.memo,
        };
        log::debug!("query: {}, pair: {}, no: {}, memo: {}", q, m.pair, m.no, m.memo);

        tx.exec_drop(q,p)?;

        Ok(())
    }

    fn select_forecast_model(&self, tx: &mut Transaction, pair: &str, no:i32) -> MyResult<Option<ForecastModel>> {
        let q = format!(
            "SELECT pair, model_no, model_data, memo, created_at, updated_at FROM {} WHERE pair = :pair AND model_no = :no",
            TABLE_NAME_FORECAST_MODEL
        );
        let p = params! {
            "pair" => pair,
            "no" => no,
        };
        log::debug!("query: {}, pair: {}, no: {}", q, pair, no);

        if let Some((pair, model_no, model_data, memo, created_at, updated_at)) = tx.exec_first(q, p)? {
            Ok(Some(ForecastModel{
                pair,
                no:model_no,
                data: model_data,
                memo,
                created_at,
                updated_at,
            }))
        } else {
            Ok(None)
        }
    }
}
