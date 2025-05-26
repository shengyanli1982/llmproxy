use reqwest_middleware::Error as ReqwestMiddlewareError;
use std::io;
use thiserror::Error;

/// 应用错误类型
#[derive(Error, Debug)]
pub enum AppError {
    /// IO 错误
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// 配置错误
    #[error("Configuration error: {0}")]
    Config(String),

    /// HTTP客户端错误
    #[error("HTTP client error: {0}")]
    HttpError(#[from] reqwest::Error),

    /// HTTP中间件错误
    #[error("HTTP middleware error: {0}")]
    HttpMiddlewareError(#[from] ReqwestMiddlewareError),

    /// 上游错误
    #[error("Upstream error: {0}")]
    Upstream(String),

    /// 上游组不存在
    #[error("Upstream group not found: {0}")]
    UpstreamGroupNotFound(String),

    /// 无可用上游
    #[error("No upstream available")]
    NoUpstreamAvailable,

    /// 无效代理配置
    #[error("Invalid proxy configuration: {0}")]
    InvalidProxy(String),

    /// 路由错误
    #[error("Routing error: {0}")]
    Routing(String),

    /// 内部错误
    #[error("Internal error: {0}")]
    Internal(String),

    /// 序列化/反序列化错误
    #[error("Serialization/deserialization error: {0}")]
    SerdeError(#[from] serde_json::Error),

    /// 请求验证错误
    #[error("Request validation error: {0}")]
    ValidationError(String),

    /// 无效的HTTP头
    #[error("Invalid HTTP header: {0}")]
    InvalidHeader(String),

    /// 认证错误
    #[error("Authentication error: {0}")]
    AuthError(String),
}
