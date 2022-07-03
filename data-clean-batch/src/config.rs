use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub expire_date_count: i64,
    pub cron_schedule: String,
}
