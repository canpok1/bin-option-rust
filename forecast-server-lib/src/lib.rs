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
pub enum ForecastAfter5minHistoryIdGetResponse {
    /// 登録成功
    Status200
    (models::ForecastAfter5minHistoryIdGet200Response)
    ,
    /// 登録失敗（レート情報が見つからない）
    Status404
    (models::Error)
    ,
    /// 登録失敗（内部エラー）
    Status500
    (models::Error)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum HistoriesPostResponse {
    /// 登録成功
    Status201
    (models::HistoriesPost201Response)
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
    async fn forecast_after5min_history_id_get(
        &self,
        history_id: String,
        context: &C) -> Result<ForecastAfter5minHistoryIdGetResponse, ApiError>;

    /// レート履歴を新規登録します
    async fn histories_post(
        &self,
        history: models::History,
        context: &C) -> Result<HistoriesPostResponse, ApiError>;

}

/// API where `Context` isn't passed on every API call
#[async_trait]
pub trait ApiNoContext<C: Send + Sync> {

    fn poll_ready(&self, _cx: &mut Context) -> Poll<Result<(), Box<dyn Error + Send + Sync + 'static>>>;

    fn context(&self) -> &C;

    /// 5分後の予想を取得します
    async fn forecast_after5min_history_id_get(
        &self,
        history_id: String,
        ) -> Result<ForecastAfter5minHistoryIdGetResponse, ApiError>;

    /// レート履歴を新規登録します
    async fn histories_post(
        &self,
        history: models::History,
        ) -> Result<HistoriesPostResponse, ApiError>;

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
    async fn forecast_after5min_history_id_get(
        &self,
        history_id: String,
        ) -> Result<ForecastAfter5minHistoryIdGetResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().forecast_after5min_history_id_get(history_id, &context).await
    }

    /// レート履歴を新規登録します
    async fn histories_post(
        &self,
        history: models::History,
        ) -> Result<HistoriesPostResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().histories_post(history, &context).await
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
