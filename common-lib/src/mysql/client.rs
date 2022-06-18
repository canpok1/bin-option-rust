use mysql::{params, prelude::Queryable, OptsBuilder, Pool, PooledConn};

use crate::error::MyResult;

use super::model::RateForTraining;

pub trait Client {
    fn insert_rates_for_training(&self, rates: &Vec<RateForTraining>) -> MyResult<()>;
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

    fn get_conn(&self) -> MyResult<PooledConn> {
        match self.pool.get_conn() {
            Ok(v) => Ok(v),
            Err(e) => Err(Box::new(e)),
        }
    }
}

impl Client for DefaultClient {
    fn insert_rates_for_training(&self, rates: &Vec<RateForTraining>) -> MyResult<()> {
        let mut conn = self.get_conn()?;

        conn.exec_batch(
            "INSERT INTO rates_for_training (pair, recorded_at, rate) VALUES (:pair, :recorded_at, :rate);",
            rates.iter().map(|rate| params! {
                "pair" => &rate.pair,
                "recorded_at" => rate.recored_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                "rate" => &rate.rate,
            })
        )?;

        Ok(())
    }
}
