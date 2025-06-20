use crate::{
    config::{
        common::{ProxyConfig, RetryConfig},
        defaults::{
            default_connect_timeout, default_idle_timeout, default_keepalive,
            default_request_timeout,
        },
        validation,
    },
    r#const::http_client_limits,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// HTTP客户端配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[validate(schema(function = "validation::validate_http_client_config"))]
#[serde(rename_all = "lowercase")]
pub struct HttpClientConfig {
    /// 超时配置
    #[serde(default)]
    #[validate(nested)]
    pub timeout: HttpClientTimeoutConfig,
    /// TCP Keepalive
    #[serde(default = "default_keepalive")]
    #[validate(range(
        min = "http_client_limits::MIN_KEEPALIVE",
        max = "http_client_limits::MAX_KEEPALIVE"
    ))]
    pub keepalive: u32,
    /// 重试配置
    #[serde(default)]
    #[validate(nested)]
    pub retry: Option<RetryConfig>,
    /// 代理配置
    #[serde(default)]
    #[validate(nested)]
    pub proxy: Option<ProxyConfig>,
    /// 是否启用流式模式
    #[serde(default)]
    pub stream_mode: bool,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout: HttpClientTimeoutConfig::default(),
            keepalive: default_keepalive(),
            retry: None,
            proxy: None,
            stream_mode: false,
        }
    }
}

/// HTTP客户端超时配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "lowercase")]
pub struct HttpClientTimeoutConfig {
    /// 连接超时（秒）
    #[serde(default = "default_connect_timeout")]
    #[validate(range(
        min = "http_client_limits::MIN_CONNECT_TIMEOUT",
        max = "http_client_limits::MAX_CONNECT_TIMEOUT"
    ))]
    pub connect: u64,
    /// 请求超时（秒）
    #[serde(default = "default_request_timeout")]
    #[validate(range(
        min = "http_client_limits::MIN_REQUEST_TIMEOUT",
        max = "http_client_limits::MAX_REQUEST_TIMEOUT"
    ))]
    pub request: u64,
    /// 空闲连接超时（秒）
    #[serde(default = "default_idle_timeout")]
    #[validate(range(
        min = "http_client_limits::MIN_IDLE_TIMEOUT",
        max = "http_client_limits::MAX_IDLE_TIMEOUT"
    ))]
    pub idle: u64,
}

impl Default for HttpClientTimeoutConfig {
    fn default() -> Self {
        Self {
            connect: default_connect_timeout(),
            request: default_request_timeout(),
            idle: default_idle_timeout(),
        }
    }
}
