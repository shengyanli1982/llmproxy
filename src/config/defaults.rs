use crate::r#const::{
    breaker_limits, http_client_limits, rate_limit_limits, retry_limits, weight_limits,
};
use uuid::Uuid;

// 默认监听地址
pub fn default_listen_address() -> String {
    "0.0.0.0".to_string()
}

// 默认管理服务端口
pub fn default_admin_port() -> u16 {
    9000
}

// 默认转发服务端口
pub fn default_listen_port() -> u16 {
    3000
}

// 默认连接超时（秒）
pub fn default_connect_timeout() -> u64 {
    http_client_limits::DEFAULT_CONNECT_TIMEOUT
}

// 默认请求超时（秒）
pub fn default_request_timeout() -> u64 {
    http_client_limits::DEFAULT_REQUEST_TIMEOUT
}

// 默认空闲超时（秒）
pub fn default_idle_timeout() -> u64 {
    http_client_limits::DEFAULT_IDLE_TIMEOUT
}

// 默认 keepalive 时间（秒）
pub fn default_keepalive() -> u32 {
    http_client_limits::DEFAULT_KEEPALIVE
}

// 默认用户代理
pub fn default_user_agent() -> String {
    "LLMProxy/1.0".to_string()
}

// 默认重试次数
pub fn default_retry_attempts() -> u32 {
    retry_limits::DEFAULT_ATTEMPTS
}

// 默认初始重试间隔（毫秒）
pub fn default_retry_initial() -> u32 {
    retry_limits::DEFAULT_INITIAL_MS
}

// 默认权重
pub fn default_weight() -> u32 {
    weight_limits::MIN_WEIGHT
}

// 默认每秒请求数
pub fn default_per_second() -> u32 {
    rate_limit_limits::DEFAULT_PER_SECOND
}

// 默认突发请求上限
pub fn default_burst() -> u32 {
    rate_limit_limits::DEFAULT_BURST
}

// 默认流式模式
pub fn default_stream_mode() -> bool {
    true // 默认启用流式响应支持
}

// 默认 UUID v4 字符串
pub fn default_uuid_v4_string() -> String {
    Uuid::new_v4().to_string() // 生成默认的 UUID v4 字符串
}

// 熔断器默认阈值
pub fn default_circuitbreaker_threshold() -> f64 {
    breaker_limits::DEFAULT_THRESHOLD
}

// 熔断器默认冷却时间（秒）
pub fn default_circuitbreaker_cooldown() -> u64 {
    breaker_limits::DEFAULT_COOLDOWN
}
