use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    // 共通設定
    pub forecast_input_size: usize,
    pub forecast_offset_minutes: usize,
    pub currency_pair: String,

    // 定期実行スケジュール（定期実行しない場合は空文字）
    pub cron_schedule: String,
    // 予測用モデルに割り当てる番号
    pub forecast_model_no: i32,
    // 学習中モデルに割り当てる番号
    pub training_model_no: i32,
    // 1世代あたりのモデル数
    pub training_model_count: usize,
    // 最大世代数
    pub generation_count: i32,

    // 学習データの必要数
    pub training_data_required_count: usize,
    // 学習データ取得範囲（開始）の算出用オフセット値（現在日時から何時間前にするかを指定）
    pub training_data_range_begin_offset_hour: i64,
    // 学習データ取得範囲（終了）の算出用オフセット値（現在日時から何時間前にするかを指定）
    pub training_data_range_end_offset_hour: i64,

    // テストデータの必要数
    pub test_data_required_count: usize,
    // テストデータ取得範囲（開始）の算出用オフセット値（現在日時から何時間前にするかを指定）
    pub test_data_range_begin_offset_hour: i64,
    // テストデータ取得範囲（終了）の算出用オフセット値（現在日時から何時間前にするかを指定）
    pub test_data_range_end_offset_hour: i64,

    // 交叉率
    pub crossover_rate: f32,
    // 突然変異率
    pub mutation_rate: f32,
}
