use crate::config::defaults::{
    default_burst, default_circuitbreaker_cooldown, default_circuitbreaker_threshold,
    default_connect_timeout, default_per_second, default_retry_attempts, default_retry_initial,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// 超时配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimeoutConfig {
    // 连接超时（秒）
    #[serde(default = "default_connect_timeout")]
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
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RateLimitConfig {
    // 是否启用限流
    #[serde(default)]
    pub enabled: bool,
    // 每秒请求数
    #[serde(default = "default_per_second")]
    pub per_second: u32,
    // 突发请求上限
    #[serde(default = "default_burst")]
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
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RetryConfig {
    // 是否启用重试
    #[serde(default)]
    pub enabled: bool,
    // 最大重试次数
    #[serde(default = "default_retry_attempts")]
    pub attempts: u32,
    // 初始重试间隔（毫秒）
    #[serde(default = "default_retry_initial")]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct ProxyConfig {
    // 是否启用代理
    #[serde(default)]
    pub enabled: bool,
    // 代理URL
    #[serde(default)]
    pub url: String,
}

// 熔断器配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BreakerConfig {
    // 触发熔断的失败率阈值 (0.01-1.0, 例如0.5表示50%的调用失败)
    #[serde(default = "default_circuitbreaker_threshold")]
    pub threshold: f64,
    // 熔断开启后进入半开 (Half-Open) 状态的冷却时间 (秒)
    #[serde(default = "default_circuitbreaker_cooldown")]
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
