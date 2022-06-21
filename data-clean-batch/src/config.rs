use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    // DB関連
    pub db_host: String,
    pub db_port: u16,
    pub db_name: String,
    pub db_user_name: String,
    pub db_password: String,

    // バッチ関連
    pub expire_date_count: i64,
    pub cron_schedule: String,
}