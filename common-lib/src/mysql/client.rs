use chrono::NaiveDateTime;
use mysql::{params, prelude::Queryable, OptsBuilder, Pool, TxOpts, Transaction};

use crate::error::MyResult;

use super::model::RateForTraining;

pub trait Client
{
    fn with_transaction<F>(&self, f: F) -> MyResult<()>
    where
        F: FnMut(&mut Transaction) -> MyResult<()>
    ;
    fn insert_rates_for_training(&self, tx: &mut Transaction, rates: &Vec<RateForTraining>) -> MyResult<()>;
    fn delete_old_rates_for_training(&self, tx: &mut Transaction, border: &NaiveDateTime) -> MyResult<()>;
    fn select_rates_for_training(&self, tx: &mut Transaction, pair: &str, begin: Option<NaiveDateTime>, end: Option<NaiveDateTime>) -> MyResult<Vec<RateForTraining>>;
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
                RateForTraining::get_table_name()
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
                RateForTraining::get_table_name()
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
            where_str = format!("{} AND {}", where_str, conditions.join(" "));
        };

        let query = format!(
            "SELECT pair, recorded_at, rate, created_at, updated_at FROM {} {} ORDER BY recorded_at ASC",
            RateForTraining::get_table_name(),
            where_str,
        );
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
}
