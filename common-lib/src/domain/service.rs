use ta::{indicators::MovingAverageConvergenceDivergence, Next};

use crate::error::MyResult;

pub struct Converter {}

pub struct Param {
    pub fast_period: usize,
    pub slow_period: usize,
    pub signal_period: usize,
}

impl Converter {
    // 入力データの1ブロックサイズ
    const INPUT_DATA_BLOCK_SIZE: usize = 10;

    pub fn convert_to_input_data(&self, rates_org: &Vec<f64>, p: &Param) -> MyResult<Vec<f64>> {
        let size = rates_org.len();

        let mut macd =
            MovingAverageConvergenceDivergence::new(p.fast_period, p.slow_period, p.signal_period)?;

        // ブロック1-4の順に配列に格納
        // ブロック1: レート
        // ブロック2: MACD
        // ブロック3: signal
        // ブロック4: histogram
        let mut rates = vec![];
        let mut macds = vec![];
        let mut signals = vec![];
        let mut histograms = vec![];
        for (i, rate) in rates_org.iter().enumerate() {
            let output = macd.next(*rate);
            if i >= size - Self::INPUT_DATA_BLOCK_SIZE {
                rates.push(*rate);
                macds.push(output.macd);
                signals.push(output.signal);
                histograms.push(output.histogram);
            }
        }

        let mut converted = vec![];
        converted.extend(&rates);
        converted.extend(&macds);
        converted.extend(&signals);
        converted.extend(&histograms);
        Ok(converted)
    }
}
