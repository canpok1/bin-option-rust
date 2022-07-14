use ta::{
    indicators::{BollingerBands, MovingAverageConvergenceDivergence},
    Next,
};

use crate::error::MyResult;

use super::model::{FeatureData, FeatureParams, InputData};

pub fn convert_to_feature(rates_org: &InputData, p: &FeatureParams) -> MyResult<FeatureData> {
    let size = rates_org.len();

    let mut macd =
        MovingAverageConvergenceDivergence::new(p.fast_period, p.slow_period, p.signal_period)?;
    let mut bb = BollingerBands::new(p.bb_period, 2.0_f64)?;

    // 特徴量1から順に配列へと格納
    // 特徴量1: レート
    // 特徴量2: MACD（histogram）
    // 特徴量3: BB（Upper）
    // 特徴量4: BB（Lower）
    let mut rates = vec![];
    let mut histograms = vec![];
    let mut bb_uppers = vec![];
    let mut bb_lowers = vec![];
    for (i, rate) in rates_org.iter().enumerate() {
        let macd_output = macd.next(*rate);
        let bb_output = bb.next(*rate);
        if i >= size - p.feature_size {
            rates.push(*rate);

            histograms.push(macd_output.histogram);

            bb_uppers.push(bb_output.upper);
            bb_lowers.push(bb_output.lower);
        }
    }

    let mut converted = vec![];
    converted.extend(&rates);
    converted.extend(&histograms);
    converted.extend(&bb_uppers);
    converted.extend(&bb_lowers);
    Ok(converted)
}

pub fn convert_to_features(
    inputs: &Vec<InputData>,
    p: &FeatureParams,
) -> MyResult<Vec<FeatureData>> {
    let mut features = vec![];

    for input in inputs.iter() {
        let f = convert_to_feature(input, p)?;
        features.push(f);
    }

    Ok(features)
}
