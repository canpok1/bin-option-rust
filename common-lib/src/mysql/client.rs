use chrono::NaiveDateTime;
use mysql::{params, prelude::Queryable, OptsBuilder, Pool, PooledConn, TxOpts, Transaction};

use crate::error::MyResult;

use super::model::RateForTraining;

pub trait Client
{
    fn with_transaction<F>(&self, f: F) -> MyResult<()>
    where
        F: Fn(&mut Transaction) -> MyResult<()>
    ;
    fn insert_rates_for_training(&self, tx: &mut Transaction, rates: &Vec<RateForTraining>) -> MyResult<()>;
    fn delete_old_rates_for_training(&self, tx: &mut Transaction, border: &NaiveDateTime) -> MyResult<()>;
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
    /// # Example
    /// ```
    /// match client.with_transaction(
    ///     |tx| -> MyResult<()> {
    ///         tx.exec_batch(...)?;
    ///         Ok(())
    ///     }
    /// ) {
    ///     Ok(_) => { ... }
    ///     Err(err) => { ... }
    /// };
    /// ```
    fn with_transaction<F>(&self, f: F) -> MyResult<()>
    where
        F: Fn(&mut Transaction) -> MyResult<()>
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
                    "recorded_at" => rate.recored_at.format("%Y-%m-%d %H:%M:%S").to_string(),
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
}
