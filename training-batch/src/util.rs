use std::collections::HashSet;

use chrono::NaiveDateTime;
use common_lib::{
    domain::model::InputData,
    error::MyResult,
    mysql::client::{Client, DefaultClient},
};
use log::debug;
use rand::Rng;

use crate::config;

pub fn load_input_data(
    config: &config::Config,
    mysql_cli: &DefaultClient,
    begin: NaiveDateTime,
    end: NaiveDateTime,
) -> MyResult<(Vec<InputData>, Vec<f64>)> {
    let mut x: Vec<InputData> = vec![];
    let mut y: Vec<f64> = vec![];

    mysql_cli.with_transaction(|tx| -> MyResult<()> {
        debug!("fetch rates. begin:{}, end:{}", begin, end);

        let rates = mysql_cli.select_rates_for_training(
            tx,
            &config.currency_pair,
            Some(begin),
            Some(end),
        )?;
        debug!("fetched rates count: {}", rates.len());

        for offset in 0..rates.len() {
            // 似たようなデータを減らすために期間を空ける
            if offset % 5 > 0 {
                continue;
            }

            let truth =
                rates.get(offset + config.forecast_input_size - 1 + config.forecast_offset_minutes);
            if truth.is_none() {
                break;
            }

            let mut before: f64 = 0.0;
            let mut same_count = 0;
            let mut data: Vec<f64> = vec![];
            for index in offset..offset + config.forecast_input_size {
                data.push(rates[index].rate.clone());
                if rates[index].rate == before {
                    same_count += 1;
                }
                before = rates[index].rate.clone();
            }

            // 長期間変動がないデータは学習データとしては不適切なのでスキップ
            if same_count > (data.len() / 2) {
                continue;
            }

            x.push(data);
            y.push(truth.unwrap().rate);
        }

        Ok(())
    })?;
    Ok((x, y))
}

pub fn train_test_split(
    x: &Vec<InputData>,
    y: &Vec<f64>,
    test_ratio: f32,
) -> MyResult<(Vec<InputData>, Vec<InputData>, Vec<f64>, Vec<f64>)> {
    let mut test_indexes = HashSet::new();
    let mut rng = rand::thread_rng();

    for i in 0..x.len() {
        if rng.gen::<f32>() <= test_ratio {
            test_indexes.insert(i);
        }
    }

    let mut train_x = vec![];
    let mut train_y = vec![];
    let mut test_x = vec![];
    let mut test_y = vec![];
    for i in 0..x.len() {
        if test_indexes.contains(&i) {
            test_x.push(x[i].clone());
            test_y.push(y[i]);
        } else {
            train_x.push(x[i].clone());
            train_y.push(y[i]);
        }
    }

    Ok((train_x, test_x, train_y, test_y))
}
