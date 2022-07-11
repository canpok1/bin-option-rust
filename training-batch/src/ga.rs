use std::cmp;

use common_lib::{domain::model::FeatureParams, error::MyResult};
use rand::Rng;

use crate::config;

#[derive(Clone)]
pub struct Gene {
    values: Vec<usize>,
}

impl Gene {
    const FEATURE_SIZE: usize = 10;
    const MIN_VALUE: usize = 2;

    pub fn new(p: &FeatureParams) -> MyResult<Gene> {
        let mut values = vec![];
        values.push(p.fast_period);
        values.push(p.slow_period);
        values.push(p.signal_period);
        Ok(Gene { values })
    }

    pub fn new_random_gene(config: &config::Config) -> MyResult<Gene> {
        let mut rng = rand::thread_rng();
        Ok(Gene {
            values: vec![
                rng.gen_range(Self::MIN_VALUE..=config.forecast_input_size),
                rng.gen_range(Self::MIN_VALUE..=config.forecast_input_size),
                rng.gen_range(Self::MIN_VALUE..=config.forecast_input_size),
            ],
        })
    }

    pub fn to_feature_params(&self) -> MyResult<FeatureParams> {
        Ok(FeatureParams {
            feature_size: Self::FEATURE_SIZE,
            fast_period: self.values[0],
            slow_period: self.values[1],
            signal_period: self.values[2],
        })
    }

    pub fn mutation(&mut self, config: &config::Config) -> MyResult<()> {
        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..self.values.len());
        self.values[index] = rng.gen_range(Self::MIN_VALUE..=config.forecast_input_size);
        Ok(())
    }

    pub fn select_index_random(genes: &Vec<Gene>) -> MyResult<usize> {
        let mut rng = rand::thread_rng();
        Ok(rng.gen_range(0..genes.len()))
    }

    pub fn select_index_roulette(weights: &Vec<f64>) -> MyResult<usize> {
        let total: f64 = weights.iter().map(|v| 1.0 - v).sum();

        let mut rng = rand::thread_rng();
        let border: f64 = rng.gen();
        let mut sum: f64 = 0.0;
        let mut index: usize = 0;
        for (i, w) in weights.iter().enumerate() {
            if i == weights.len() {
                index = i;
                break;
            }

            sum += w / total;
            if sum >= border {
                index = i;
                break;
            }
        }
        Ok(index)
    }

    pub fn crossover(g1: &mut Self, g2: &mut Self, max: usize) -> MyResult<()> {
        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..g1.values.len());
        let mask = 3 << rng.gen_range(0..3);

        let tmp1 = g1.values[index] & mask;
        let tmp2 = g2.values[index] & mask;

        g1.values[index] = (g1.values[index] & !mask) | tmp2;
        g1.values[index] = cmp::min(g1.values[index], max);
        g1.values[index] = cmp::max(g1.values[index], Self::MIN_VALUE);

        g2.values[index] = (g2.values[index] & !mask) | tmp1;
        g2.values[index] = cmp::min(g2.values[index], max);
        g2.values[index] = cmp::max(g2.values[index], Self::MIN_VALUE);
        Ok(())
    }
}
