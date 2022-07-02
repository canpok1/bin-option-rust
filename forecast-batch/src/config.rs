use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    // 共通設定
    pub forecast_input_size: usize,
    pub forecast_offset_minutes: usize,
    pub currency_pair: String,

    // バッチ関連
    pub cron_schedule: String,
}