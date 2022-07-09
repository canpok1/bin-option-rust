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

    #[error("unknown model type, value:{}", value)]
    UnknownModelType { value: u8 },

    #[error("unsupported model type enum, value:{}", value)]
    UnsupportedModelTypeEnum { value: String },

    #[error("unmatch feature params hash, pair:{}, model_no:{}", pair, model_no)]
    UnmatchFeatureParamsHash { pair: String, model_no: i32 },
}
