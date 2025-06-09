use crate::config::common::{ProxyConfig, RetryConfig};
use crate::config::defaults::{
    default_connect_timeout, default_idle_timeout, default_keepalive, default_request_timeout,
    default_stream_mode, default_user_agent,
};
use serde::{Deserialize, Serialize};

// HTTP客户端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpClientConfig {
    // 用户代理
    #[serde(default = "default_user_agent")]
    pub agent: String,
    // TCP Keepalive（秒）
    #[serde(default = "default_keepalive")]
    pub keepalive: u32,
    // 超时配置
    #[serde(default)]
    pub timeout: HttpClientTimeoutConfig,
    // 重试配置
    #[serde(default)]
    pub retry: RetryConfig,
    // 代理配置
    #[serde(default)]
    pub proxy: ProxyConfig,
    // 是否支持流式响应（如果为true，则不设置请求超时）
    #[serde(default = "default_stream_mode", rename = "stream")]
    pub stream_mode: bool,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            agent: default_user_agent(),
            keepalive: default_keepalive(),
            timeout: HttpClientTimeoutConfig::default(),
            retry: RetryConfig::default(),
            proxy: ProxyConfig::default(),
            stream_mode: default_stream_mode(),
        }
    }
}

// HTTP客户端超时配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpClientTimeoutConfig {
    // 连接超时（秒）
    #[serde(default = "default_connect_timeout")]
    pub connect: u64,
    // 请求超时（秒）
    #[serde(default = "default_request_timeout")]
    pub request: u64,
    // 空闲超时（秒）
    #[serde(default = "default_idle_timeout")]
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
