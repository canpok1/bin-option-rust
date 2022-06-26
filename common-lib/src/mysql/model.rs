use chrono::NaiveDate;
use smartcore::{
    linalg::Matrix, 
    ensemble::random_forest_regressor::RandomForestRegressor
};

use crate::{error::MyResult, domain};


#[derive(Debug, Clone)]
pub struct ForecastModel {
    pub pair: String,
    pub no: i32,
    pub data: Vec<u8>,
    pub memo: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl ForecastModel
{
    pub fn new(pair: String, no: i32, data: Vec<u8>, memo: String) -> MyResult<ForecastModel> {
        let dummy = NaiveDate::from_ymd(2022, 1, 1).and_hms(0, 0, 0);
        Ok(ForecastModel {
            pair: pair,
            no: no,
            data: data,
            memo: memo,
            created_at: dummy.clone(),
            updated_at: dummy.clone(),
        })
    }

    pub fn to_domain<M>(&self) -> MyResult<domain::model::ForecastModel<M, RandomForestRegressor<f64>>>
    where
        M: Matrix<f64>
    {
        let m = domain::model::ForecastModel::new(
            self.pair.clone(),
            self.no,
            Box::new(bincode::deserialize::<RandomForestRegressor<f64>>(&self.data)?),
            self.memo.clone(),
        )?;
        Ok(m)
    }
}
