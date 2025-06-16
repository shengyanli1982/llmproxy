use crate::{
    config::{
        defaults::{
            default_burst, default_circuitbreaker_cooldown, default_circuitbreaker_threshold,
            default_connect_timeout, default_per_second, default_retry_attempts,
            default_retry_initial,
        },
        validation,
    },
    r#const::{breaker_limits, http_client_limits, rate_limit_limits, retry_limits},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

// 超时配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "lowercase")]
pub struct TimeoutConfig {
    // 连接超时（秒）
    #[serde(default = "default_connect_timeout")]
    #[validate(range(
        min = "http_client_limits::MIN_CONNECT_TIMEOUT",
        max = "http_client_limits::MAX_CONNECT_TIMEOUT"
    ))]
    pub connect: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect: default_connect_timeout(),
        }
    }
}

// 限流配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "lowercase")]
pub struct RateLimitConfig {
    // 是否启用限流
    #[serde(default)]
    pub enabled: bool,
    // 每秒请求数
    #[serde(default = "default_per_second")]
    #[validate(range(
        min = "rate_limit_limits::MIN_PER_SECOND",
        max = "rate_limit_limits::MAX_PER_SECOND"
    ))]
    pub per_second: u32,
    // 突发请求上限
    #[serde(default = "default_burst")]
    #[validate(range(
        min = "rate_limit_limits::MIN_BURST",
        max = "rate_limit_limits::MAX_BURST"
    ))]
    pub burst: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            per_second: default_per_second(),
            burst: default_burst(),
        }
    }
}

// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "lowercase")]
pub struct RetryConfig {
    // 是否启用重试
    #[serde(default)]
    pub enabled: bool,
    // 最大重试次数
    #[serde(default = "default_retry_attempts")]
    #[validate(range(min = "retry_limits::MIN_ATTEMPTS", max = "retry_limits::MAX_ATTEMPTS"))]
    pub attempts: u32,
    // 初始重试间隔（毫秒）
    #[serde(default = "default_retry_initial")]
    #[validate(range(
        min = "retry_limits::MIN_INITIAL_MS",
        max = "retry_limits::MAX_INITIAL_MS"
    ))]
    pub initial: u32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            attempts: default_retry_attempts(),
            initial: default_retry_initial(),
        }
    }
}

// 代理配置
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema, Validate)]
#[validate(schema(function = "validation::validate_proxy_config"))]
#[serde(rename_all = "lowercase")]
pub struct ProxyConfig {
    // 是否启用代理
    #[serde(default)]
    pub enabled: bool,
    // 代理URL
    #[serde(default)]
    pub url: String,
}

// 熔断器配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "lowercase")]
pub struct BreakerConfig {
    // 触发熔断的失败率阈值 (0.01-1.0, 例如0.5表示50%的调用失败)
    #[serde(default = "default_circuitbreaker_threshold")]
    #[validate(range(
        min = "breaker_limits::MIN_THRESHOLD",
        max = "breaker_limits::MAX_THRESHOLD"
    ))]
    pub threshold: f64,
    // 熔断开启后进入半开 (Half-Open) 状态的冷却时间 (秒)
    #[serde(default = "default_circuitbreaker_cooldown")]
    #[validate(range(
        min = "breaker_limits::MIN_COOLDOWN",
        max = "breaker_limits::MAX_COOLDOWN"
    ))]
    pub cooldown: u64,
}

impl Default for BreakerConfig {
    fn default() -> Self {
        Self {
            threshold: default_circuitbreaker_threshold(),
            cooldown: default_circuitbreaker_cooldown(),
        }
    }
}
