use chrono::NaiveDateTime;
use mysql::{params, prelude::Queryable, OptsBuilder, Pool, PooledConn};

use crate::error::MyResult;

use super::model::RateForTraining;

pub trait Client<T>
where
    T: Queryable,
{
    fn insert_rates_for_training(&self, tx: &mut T, rates: &Vec<RateForTraining>) -> MyResult<()>;
    fn delete_old_rates_for_training(&self, tx: &mut T, border: &NaiveDateTime) -> MyResult<()>;
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

    pub fn get_conn(&self) -> MyResult<PooledConn> {
        match self.pool.get_conn() {
            Ok(v) => Ok(v),
            Err(e) => Err(Box::new(e)),
        }
    }
}

impl<T> Client<T> for DefaultClient
where
    T: Queryable,
{
    fn insert_rates_for_training(&self, tx: &mut T, rates: &Vec<RateForTraining>) -> MyResult<()> {
        tx.exec_batch(
            format!(
                "INSERT INTO {} (pair, recorded_at, rate) VALUES (:pair, :recorded_at, :rate);",
                RateForTraining::get_table_name()
            ),
            rates.iter().map(|rate| {
                params! {
                    "pair" => &rate.pair,
                    "recorded_at" => rate.recored_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                    "rate" => &rate.rate,
                }
            }),
        )?;

        Ok(())
    }

    fn delete_old_rates_for_training(&self, tx: &mut T, border: &NaiveDateTime) -> MyResult<()> {
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
}
