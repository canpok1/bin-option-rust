use chrono::{NaiveDate, NaiveDateTime};

use crate::error::{MyError, MyResult};

#[derive(Debug, Clone)]
pub struct RateForTraining {
    pub pair: String,
    pub recorded_at: chrono::NaiveDateTime,
    pub rate: f64,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl RateForTraining {
    pub fn new(pair: &str, time: &str, rate: f64) -> MyResult<RateForTraining> {
        let recored_at: NaiveDateTime;
        match NaiveDateTime::parse_from_str(time, "%Y-%m-%d %H:%M:%S") {
            Ok(v) => {
                recored_at = v;
            }
            Err(err) => {
                return Err(Box::new(MyError::ParseError {
                    param_name: "time".to_string(),
                    value: time.to_string(),
                    memo: format!("{}", err),
                }));
            }
        }
        Ok(RateForTraining {
            pair: pair.to_string(),
            recorded_at: recored_at,
            rate: rate,
            created_at: NaiveDate::from_ymd(2022, 1, 1).and_hms(0, 0, 0),
            updated_at: NaiveDate::from_ymd(2022, 1, 1).and_hms(0, 0, 0),
        })
    }
}


#[derive(Debug, Clone)]
pub struct ForecastModel {
    pub pair: String,
    pub no: i32,
    pub data: Vec<u8>,
    pub memo: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl ForecastModel {
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
}
