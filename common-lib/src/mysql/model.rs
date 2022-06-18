use chrono::{NaiveDate, NaiveDateTime};

use crate::error::{MyError, MyResult};

#[derive(Debug, Clone)]
pub struct RateForTraining {
    pub pair: String,
    pub recored_at: chrono::NaiveDateTime,
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
            recored_at: recored_at,
            rate: rate,
            created_at: NaiveDate::from_ymd(2022, 1, 1).and_hms(0, 0, 0),
            updated_at: NaiveDate::from_ymd(2022, 1, 1).and_hms(0, 0, 0),
        })
    }

    pub fn get_table_name() -> String {
        "rates_for_training".to_string()
    }
}
