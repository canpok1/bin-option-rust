use ta::{indicators::MovingAverageConvergenceDivergence, Next};

use crate::error::MyResult;

use super::model::{FeatureData, FeatureParams, InputData};

pub fn convert_to_feature(rates_org: &InputData, p: &FeatureParams) -> MyResult<FeatureData> {
    let size = rates_org.len();

    let mut macd =
        MovingAverageConvergenceDivergence::new(p.fast_period, p.slow_period, p.signal_period)?;

    // 特徴量1-4の順に配列に格納
    // 特徴量1: レート
    // 特徴量2: MACD
    // 特徴量3: signal
    // 特徴量4: histogram
    let mut rates = vec![];
    let mut macds = vec![];
    let mut signals = vec![];
    let mut histograms = vec![];
    for (i, rate) in rates_org.iter().enumerate() {
        let output = macd.next(*rate);
        if i >= size - p.feature_size {
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
