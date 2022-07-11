use std::collections::HashSet;

use common_lib::{
    batch,
    domain::model::ForecastModel,
    error::MyResult,
    mysql::{
        self,
        client::{Client, DefaultClient},
    },
};
use ga::Gene;
use log::{error, info};
use rand::Rng;
use training::InputDataLoader;

use crate::training::ModelMaker;

mod config;
mod ga;
mod training;
mod util;

fn init_logger() {
    env_logger::init();
}

fn main() {
    init_logger();

    let config: config::Config;
    match envy::from_env::<config::Config>() {
        Ok(c) => {
            config = c;
        }
        Err(err) => {
            error!("failed to load config, error: {}", err);
            return;
        }
    }

    let mysql_cli: mysql::client::DefaultClient;
    match mysql::util::make_cli() {
        Ok(cli) => {
            mysql_cli = cli;
        }
        Err(err) => {
            error!("failed to make mysql client, error: {}", err);
            return;
        }
    }

    if let Err(err) = batch::util::start_scheduler(&config.cron_schedule, || {
        info!("start training");
        match training(&config, &mysql_cli) {
            Ok(_) => {
                info!("finished training");
            }
            Err(err) => {
                error!("failed to training, error:{}", err);
            }
        }
    }) {
        error!("failed to start scheduler, error: {}", err);
    }
}

fn training(config: &config::Config, mysql_cli: &DefaultClient) -> MyResult<()> {
    let loader = InputDataLoader { config, mysql_cli };
    let (org_x, org_y) = loader.load()?;
    let (train_base_x, test_x, train_base_y, test_y) = util::train_test_split(&org_x, &org_y, 0.2)?;

    let maker = ModelMaker {
        config,
        mysql_cli,
        train_base_x: &train_base_x,
        train_base_y: &train_base_y,
        test_x: &test_x,
        test_y: &test_y,
    };

    let mut genes: Vec<Gene> = vec![];
    if let Some(m) = maker.load_existing_model(config.forecast_model_no)? {
        let p = m.get_feature_params()?;
        let gene = Gene::new(&p)?;
        genes.push(gene);
        info!("loaded existing data, {:?}", p);
    }

    let genes_count = genes.len() as i32;
    for _ in 0..(config.forecast_model_count - genes_count) {
        genes.push(Gene::new_random_gene(config)?);
    }

    let genes_count = genes.len() as i32;
    for gen_count in 1..=config.generation_count {
        info!(
            "generation[{:<03}/{:<03}] start",
            gen_count, config.generation_count
        );

        let mut models: Vec<Vec<ForecastModel>> = vec![];
        for (i, gene) in genes.iter().enumerate() {
            let p = gene.to_feature_params()?;

            info!(
                "generation[{:<03}/{:<03}] gene[{:<02}/{:<02}] processing ... {:?}",
                gen_count,
                config.generation_count,
                i + 1,
                genes_count,
                p
            );

            models.push(maker.make_new_models(config.forecast_model_no, &p)?);
        }

        let mut best_model: Option<&ForecastModel> = None;
        let mut results: Vec<f64> = vec![];
        for models in models.iter() {
            let index = find_best_model_index(&models)?;
            if let Some(m) = models.get(index) {
                let mse = m.get_performance_mse()?;
                results.push(mse);
                if let Some(m2) = best_model {
                    if m2.get_performance_mse()? > mse {
                        best_model = Some(m);
                    }
                } else {
                    best_model = Some(m);
                }
            }
        }
        info!(
            "generation[{:<03}/{:<03}] result: {:?}",
            gen_count, config.generation_count, results
        );

        if let Some(m) = best_model {
            info!(
                "generation[{:<03}/{:<03}] best_result: {}",
                gen_count,
                config.generation_count,
                m.get_performance_mse()?,
            );
            save_model(mysql_cli, m)?;
        }

        if gen_count == config.generation_count {
            break;
        }

        // 次世代を生成
        let mut new_genes: Vec<Gene> = vec![];
        let mut selected: HashSet<usize> = HashSet::new();
        while new_genes.len() < genes.len() {
            let mut rng = rand::thread_rng();
            let v: f32 = rng.gen();
            if v < config.crossover_rate {
                // 交叉する空きがあるかチェック
                if genes.len() - new_genes.len() < 2 {
                    continue;
                }

                // 交叉
                let (index1, index2) = loop {
                    let i = Gene::select_index_random(&genes)?;
                    let j = Gene::select_index_random(&genes)?;
                    if i != j {
                        break (i, j);
                    }
                };
                let mut g1 = genes[index1].clone();
                let mut g2 = genes[index2].clone();
                Gene::crossover(&mut g1, &mut g2, config.forecast_input_size)?;
                new_genes.push(g1);
                new_genes.push(g2);
            } else if v < (config.crossover_rate + config.mutation_rate) {
                // 突然変異
                let index = Gene::select_index_random(&genes)?;
                let mut new_gene = genes[index].clone();
                new_gene.mutation(config)?;
                new_genes.push(new_gene);
            } else {
                // 選択
                if selected.len() < genes.len() {
                    let index = loop {
                        let i = Gene::select_index_roulette(&results)?;
                        if !selected.contains(&i) {
                            break i;
                        }
                    };
                    new_genes.push(genes[index].clone());
                    selected.insert(index);
                }
            }
        }
        genes = new_genes;
    }

    Ok(())
}

fn find_best_model_index(models: &Vec<ForecastModel>) -> MyResult<usize> {
    let mut best_model_index: usize = 0;
    let mut best_mse: Option<f64> = None;
    for (i, m) in models.iter().enumerate() {
        let mse = m.get_performance_mse()?;
        if best_mse.is_none() || mse < best_mse.unwrap() {
            best_model_index = i;
            best_mse = Some(mse);
        }
    }
    Ok(best_model_index)
}

// fn load_data(
//     config: &config::Config,
//     mysql_cli: &DefaultClient,
//     begin: NaiveDateTime,
//     end: NaiveDateTime,
//     params: &FeatureParams,
// ) -> MyResult<(Vec<Vec<f64>>, Vec<f64>)> {
//     let mut x: Vec<Vec<f64>> = vec![];
//     let mut y: Vec<f64> = vec![];
//
//     let converter = Converter {};
//
//     mysql_cli.with_transaction(|tx| -> MyResult<()> {
//         debug!("fetch rates. begin:{}, end:{}", begin, end);
//
//         let rates = mysql_cli.select_rates_for_training(
//             tx,
//             &config.currency_pair,
//             Some(begin),
//             Some(end),
//         )?;
//         debug!("fetched rates count: {}", rates.len());
//
//         for offset in 0..rates.len() {
//             let truth =
//                 rates.get(offset + config.forecast_input_size - 1 + config.forecast_offset_minutes);
//             if truth.is_none() {
//                 break;
//             }
//
//             let mut before: f64 = 0.0;
//             let mut same_count = 0;
//             let mut data: Vec<f64> = vec![];
//             for index in offset..offset + config.forecast_input_size {
//                 data.push(rates[index].rate.clone());
//                 if rates[index].rate == before {
//                     same_count += 1;
//                 }
//                 before = rates[index].rate.clone();
//             }
//             if same_count > (data.len() / 2) {
//                 continue;
//             }
//             // データ数を偶数にしないとLinearの学習でエラーになるようなので偶数になるよう調整
//             if offset == rates.len() && x.len() % 2 == 0 {
//                 continue;
//             }
//             x.push(converter.convert_to_features(&data, params)?);
//             y.push(truth.unwrap().rate);
//         }
//
//         Ok(())
//     })?;
//     Ok((x, y))
// }

fn save_model(mysql_cli: &DefaultClient, model: &ForecastModel) -> MyResult<()> {
    mysql_cli.with_transaction(|tx| {
        mysql_cli.upsert_forecast_model(tx, model)?;
        Ok(())
    })?;
    Ok(())
}
