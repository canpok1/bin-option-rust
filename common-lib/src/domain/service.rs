use crate::error::MyResult;

pub struct Converter {}

impl Converter {
    pub fn convert_to_input_data(&self, rates: &Vec<f64>) -> MyResult<(Vec<f64>)> {
        let mut converted = vec![];

        for rate in rates {
            converted.push(*rate);
        }

        Ok((converted))
    }
}
