use std::collections::HashSet;

use common_lib::error::MyResult;
use rand::Rng;

pub fn train_test_split(
    x: &Vec<Vec<f64>>,
    y: &Vec<f64>,
    test_ratio: f32,
) -> MyResult<(Vec<Vec<f64>>, Vec<Vec<f64>>, Vec<f64>, Vec<f64>)> {
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
