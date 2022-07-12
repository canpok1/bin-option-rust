use std::cmp;

use common_lib::{
    domain::model::FeatureParams,
    error::{MyError, MyResult},
};
use rand::Rng;

use crate::config;

#[derive(Clone)]
pub struct Gene {
    values: Vec<usize>,
}

impl Gene {
    const FEATURE_SIZE_MIN: usize = 1;
    const FEATURE_SIZE_MAX: usize = 10;
    const MIN_VALUE: usize = 2;

    pub fn new(p: &FeatureParams) -> MyResult<Gene> {
        let mut values = vec![];
        values.push(p.feature_size);
        values.push(p.fast_period);
        values.push(p.slow_period - p.fast_period);
        values.push(p.signal_period);
        values.push(p.bb_period);
        Ok(Gene { values })
    }

    pub fn new_random_gene(config: &config::Config) -> MyResult<Gene> {
        Ok(Gene {
            values: vec![
                Self::round_for_feature_size(Self::gen_value_random(config)),
                Self::gen_value_random(config),
                Self::gen_value_random(config),
                Self::gen_value_random(config),
                Self::gen_value_random(config),
            ],
        })
    }

    pub fn to_feature_params(&self) -> MyResult<FeatureParams> {
        Ok(FeatureParams {
            feature_size: Self::round_for_feature_size(self.values[0]),
            fast_period: self.values[1],
            slow_period: self.values[1] + self.values[2],
            signal_period: self.values[3],
            bb_period: self.values[4],
        })
    }

    pub fn mutation(&mut self, config: &config::Config) -> MyResult<()> {
        let index = self.gen_index_random();
        self.values[index] = Self::gen_value_random(config);
        Ok(())
    }

    fn gen_index_random(&self) -> usize {
        let mut rng = rand::thread_rng();
        rng.gen_range(0..self.values.len())
    }

    fn calc_similarity(&self, other: &Gene) -> f64 {
        let mut diff_total = 0_f64;
        for (i, v) in self.values.iter().enumerate() {
            let diff = (*v as f64) - (other.values[i] as f64);
            diff_total += diff.powf(2.0);
        }
        diff_total.sqrt()
    }

    fn round_for_feature_size(v: usize) -> usize {
        (v % (Self::FEATURE_SIZE_MAX - Self::FEATURE_SIZE_MIN)) + Self::FEATURE_SIZE_MIN
    }

    pub fn gen_value_random(config: &config::Config) -> usize {
        let mut rng = rand::thread_rng();
        rng.gen_range(Self::MIN_VALUE..=config.forecast_input_size / 3)
    }

    pub fn select_gene_index_random(genes: &Vec<Gene>) -> MyResult<usize> {
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
        let index = g1.gen_index_random();
        let mask = 3 << rand::thread_rng().gen_range(0..3);

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

    pub fn make_average_gene(genes: &Vec<Gene>) -> MyResult<Gene> {
        if genes.is_empty() {
            return Err(Box::new(MyError::ArrayIsEmpty {
                name: "genes".to_string(),
            }));
        }

        let size = genes.len();
        let mut totals = vec![0; size];
        for gene in genes.iter() {
            for (i, v) in gene.values.iter().enumerate() {
                totals[i] += v;
            }
        }

        let values = totals.iter().map(|v| v / size).collect();
        Ok(Gene { values })
    }

    pub fn calc_similarity_average(genes: &Vec<Gene>) -> MyResult<f64> {
        let avg_gene = Self::make_average_gene(genes)?;

        let total: f64 = genes.iter().map(|g| g.calc_similarity(&avg_gene)).sum();
        Ok(total / genes.len() as f64)
    }
}
