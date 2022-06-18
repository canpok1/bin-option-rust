use std::error::Error;
pub type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(thiserror::Error, Debug)]
pub enum MyError {
    #[error(
        "failed to parse, param_name:{}, value:{}, memo:{}",
        param_name,
        value,
        memo
    )]
    ParseError {
        param_name: String,
        value: String,
        memo: String,
    },
}
