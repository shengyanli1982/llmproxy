use crate::r#const::{
    breaker_limits, http_client_limits, rate_limit_limits, retry_limits, weight_limits,
};

// 熔断器默认阈值
pub fn default_circuitbreaker_threshold() -> f64 {
    breaker_limits::DEFAULT_THRESHOLD
}

// 熔断器默认冷却时间（秒）
pub fn default_circuitbreaker_cooldown() -> u64 {
    breaker_limits::DEFAULT_COOLDOWN
}

// 默认值函数
pub fn default_listen_address() -> String {
    "0.0.0.0".to_string()
}

pub fn default_admin_port() -> u16 {
    9000
}

pub fn default_listen_port() -> u16 {
    3000
}

pub fn default_connect_timeout() -> u64 {
    http_client_limits::DEFAULT_CONNECT_TIMEOUT
}

pub fn default_request_timeout() -> u64 {
    http_client_limits::DEFAULT_REQUEST_TIMEOUT
}

pub fn default_idle_timeout() -> u64 {
    http_client_limits::DEFAULT_IDLE_TIMEOUT
}

pub fn default_keepalive() -> u32 {
    http_client_limits::DEFAULT_KEEPALIVE
}

pub fn default_user_agent() -> String {
    "LLMProxy/1.0".to_string()
}

pub fn default_retry_attempts() -> u32 {
    retry_limits::DEFAULT_ATTEMPTS
}

pub fn default_retry_initial() -> u32 {
    retry_limits::DEFAULT_INITIAL_MS
}

pub fn default_weight() -> u32 {
    weight_limits::MIN_WEIGHT
}

pub fn default_per_second() -> u32 {
    rate_limit_limits::DEFAULT_PER_SECOND
}

pub fn default_burst() -> u32 {
    rate_limit_limits::DEFAULT_BURST
}

pub fn default_stream_mode() -> bool {
    true
}
