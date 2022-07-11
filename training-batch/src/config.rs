use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    // 共通設定
    pub forecast_input_size: usize,
    pub forecast_offset_minutes: usize,
    pub currency_pair: String,

    // バッチ関連
    pub cron_schedule: String,
    pub training_count: usize,
    pub training_data_required_count: usize,
    pub forecast_model_no: i32,
    pub forecast_model_count: i32,
    pub generation_count: i32,
    pub training_data_range_hour: i64,
    pub crossover_rate: f32,
    pub mutation_rate: f32,
}
