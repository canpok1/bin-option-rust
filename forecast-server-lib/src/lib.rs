#![allow(missing_docs, trivial_casts, unused_variables, unused_mut, unused_imports, unused_extern_crates, non_camel_case_types)]

use async_trait::async_trait;
use futures::Stream;
use std::error::Error;
use std::task::{Poll, Context};
use swagger::{ApiError, ContextWrapper};
use serde::{Serialize, Deserialize};

type ServiceError = Box<dyn Error + Send + Sync + 'static>;

pub const BASE_PATH: &'static str = "";
pub const API_VERSION: &'static str = "1.0.0";

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum ForecastAfter5minRateIdModelNoGetResponse {
    /// 取得成功
    Status200
    (models::ForecastAfter5minRateIdModelNoGet200Response)
    ,
    /// 取得失敗（レート情報もしくはモデルが見つからない）
    Status404
    (models::Error)
    ,
    /// 取得失敗（内部エラー）
    Status500
    (models::Error)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum RatesPostResponse {
    /// 登録成功
    Status201
    (models::RatesPost201Response)
    ,
    /// 登録失敗（リクエストパラメータ不備）
    Status400
    (models::Error)
    ,
    /// 登録失敗（通貨ペアが非対応）
    Status404
    (models::Error)
    ,
    /// 登録失敗（内部エラー）
    Status500
    (models::Error)
}

/// API
#[async_trait]
pub trait Api<C: Send + Sync> {
    fn poll_ready(&self, _cx: &mut Context) -> Poll<Result<(), Box<dyn Error + Send + Sync + 'static>>> {
        Poll::Ready(Ok(()))
    }

    /// 5分後の予想を取得します
    async fn forecast_after5min_rate_id_model_no_get(
        &self,
        rate_id: String,
        model_no: i32,
        context: &C) -> Result<ForecastAfter5minRateIdModelNoGetResponse, ApiError>;

    /// レート履歴を新規登録します
    async fn rates_post(
        &self,
        history: models::History,
        context: &C) -> Result<RatesPostResponse, ApiError>;

}

/// API where `Context` isn't passed on every API call
#[async_trait]
pub trait ApiNoContext<C: Send + Sync> {

    fn poll_ready(&self, _cx: &mut Context) -> Poll<Result<(), Box<dyn Error + Send + Sync + 'static>>>;

    fn context(&self) -> &C;

    /// 5分後の予想を取得します
    async fn forecast_after5min_rate_id_model_no_get(
        &self,
        rate_id: String,
        model_no: i32,
        ) -> Result<ForecastAfter5minRateIdModelNoGetResponse, ApiError>;

    /// レート履歴を新規登録します
    async fn rates_post(
        &self,
        history: models::History,
        ) -> Result<RatesPostResponse, ApiError>;

}

/// Trait to extend an API to make it easy to bind it to a context.
pub trait ContextWrapperExt<C: Send + Sync> where Self: Sized
{
    /// Binds this API to a context.
    fn with_context(self: Self, context: C) -> ContextWrapper<Self, C>;
}

impl<T: Api<C> + Send + Sync, C: Clone + Send + Sync> ContextWrapperExt<C> for T {
    fn with_context(self: T, context: C) -> ContextWrapper<T, C> {
         ContextWrapper::<T, C>::new(self, context)
    }
}

#[async_trait]
impl<T: Api<C> + Send + Sync, C: Clone + Send + Sync> ApiNoContext<C> for ContextWrapper<T, C> {
    fn poll_ready(&self, cx: &mut Context) -> Poll<Result<(), ServiceError>> {
        self.api().poll_ready(cx)
    }

    fn context(&self) -> &C {
        ContextWrapper::context(self)
    }

    /// 5分後の予想を取得します
    async fn forecast_after5min_rate_id_model_no_get(
        &self,
        rate_id: String,
        model_no: i32,
        ) -> Result<ForecastAfter5minRateIdModelNoGetResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().forecast_after5min_rate_id_model_no_get(rate_id, model_no, &context).await
    }

    /// レート履歴を新規登録します
    async fn rates_post(
        &self,
        history: models::History,
        ) -> Result<RatesPostResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().rates_post(history, &context).await
    }

}


#[cfg(feature = "client")]
pub mod client;

// Re-export Client as a top-level name
#[cfg(feature = "client")]
pub use client::Client;

#[cfg(feature = "server")]
pub mod server;

// Re-export router() as a top-level name
#[cfg(feature = "server")]
pub use self::server::Service;

#[cfg(feature = "server")]
pub mod context;

pub mod models;

#[cfg(any(feature = "client", feature = "server"))]
pub(crate) mod header;
