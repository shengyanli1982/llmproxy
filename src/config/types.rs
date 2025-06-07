use crate::config::defaults::*;
use crate::r#const::balance_strategy_labels;
use serde::{Deserialize, Serialize};

// 配置文件结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // HTTP服务器配置
    #[serde(default)]
    pub http_server: HttpServerConfig,
    // 上游定义
    #[serde(default)]
    pub upstreams: Vec<UpstreamConfig>,
    // 上游组定义
    #[serde(default)]
    pub upstream_groups: Vec<UpstreamGroupConfig>,
}

// HTTP服务器配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HttpServerConfig {
    // 转发服务配置
    #[serde(default)]
    pub forwards: Vec<ForwardConfig>,
    // 管理服务配置
    #[serde(default)]
    pub admin: AdminConfig,
}

// 转发服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardConfig {
    // 转发服务名称
    pub name: String,
    // 监听端口
    #[serde(default = "default_listen_port")]
    pub port: u16,
    // 监听地址
    #[serde(default = "default_listen_address")]
    pub address: String,
    // 指向的上游组名
    pub upstream_group: String,
    // 限流配置
    #[serde(default)]
    pub ratelimit: RateLimitConfig,
    // 超时配置
    #[serde(default)]
    pub timeout: TimeoutConfig,
}

// 限流配置
#[derive(Debug, Clone, Serialize, Deserialize)]
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

// 超时配置
#[derive(Debug, Clone, Serialize, Deserialize)]
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

// 管理服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    // 监听端口
    #[serde(default = "default_admin_port")]
    pub port: u16,
    // 监听地址
    #[serde(default = "default_listen_address")]
    pub address: String,
    // 超时配置
    #[serde(default)]
    pub timeout: TimeoutConfig,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            port: default_admin_port(),
            address: default_listen_address(),
            timeout: TimeoutConfig::default(),
        }
    }
}

// 上游配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamConfig {
    // 上游名称
    pub name: String,
    // 上游URL
    pub url: String,
    // 唯一标识符 (内部使用)
    #[serde(skip_serializing, default = "default_uuid_v4_string")]
    pub id: String,
    // 认证配置
    #[serde(default)]
    pub auth: Option<AuthConfig>,
    // 请求头操作
    #[serde(default)]
    pub headers: Vec<HeaderOperation>,
    // 熔断器配置
    #[serde(default)]
    pub breaker: Option<BreakerConfig>,
}

// 认证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    // 认证类型
    #[serde(default)]
    pub r#type: AuthType,
    // 认证令牌（用于Bearer认证）
    #[serde(default)]
    pub token: Option<String>,
    // 用户名（用于Basic认证）
    #[serde(default)]
    pub username: Option<String>,
    // 密码（用于Basic认证）
    #[serde(default)]
    pub password: Option<String>,
}

// 认证类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    // Bearer令牌认证
    Bearer,
    // 基本认证
    Basic,
    // 无认证
    None,
}

impl Default for AuthType {
    fn default() -> Self {
        Self::None
    }
}

// 请求头操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderOperation {
    // 操作类型
    pub op: HeaderOpType,
    // 头部名称
    pub key: String,
    // 头部值（对于insert和replace操作）
    #[serde(default)]
    pub value: Option<String>,
}

// 请求头操作类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HeaderOpType {
    // 插入（如果不存在）
    Insert,
    // 删除
    Remove,
    // 替换（如果存在）或插入（如果不存在）
    Replace,
}

// 上游组配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamGroupConfig {
    // 上游组名称
    pub name: String,
    // 上游引用列表
    pub upstreams: Vec<UpstreamRef>,
    // 负载均衡策略
    #[serde(default)]
    pub balance: BalanceConfig,
    // HTTP客户端配置
    #[serde(default)]
    pub http_client: HttpClientConfig,
}

// 上游引用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamRef {
    // 上游名称
    pub name: String,
    // 权重（用于加权轮询策略）
    #[serde(default = "default_weight")]
    pub weight: u32,
}

// 负载均衡策略配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BalanceConfig {
    // 策略类型
    #[serde(default)]
    pub strategy: BalanceStrategy,
}

// 负载均衡策略类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BalanceStrategy {
    // 轮询
    #[serde(rename = "roundrobin")]
    RoundRobin,
    // 加权轮询
    #[serde(rename = "weighted_roundrobin")]
    WeightedRoundRobin,
    // 随机
    Random,
    // 响应时间感知
    #[serde(rename = "response_aware")]
    ResponseAware,
    // 故障转移
    #[serde(rename = "failover")]
    Failover,
}

impl Default for BalanceStrategy {
    fn default() -> Self {
        Self::RoundRobin
    }
}

// 将 BalanceStrategy 转换为字符串标签
impl BalanceStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RoundRobin => balance_strategy_labels::ROUND_ROBIN,
            Self::WeightedRoundRobin => balance_strategy_labels::WEIGHTED_ROUND_ROBIN,
            Self::Random => balance_strategy_labels::RANDOM,
            Self::ResponseAware => balance_strategy_labels::RESPONSE_AWARE,
            Self::Failover => balance_strategy_labels::FAILOVER,
        }
    }
}

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

// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProxyConfig {
    // 是否启用代理
    #[serde(default)]
    pub enabled: bool,
    // 代理URL
    #[serde(default)]
    pub url: String,
}

// 熔断器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
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

// 为 Option<T> 添加 is_none_or 方法，用于配置验证
pub trait OptionExt<T> {
    fn is_none_or<F>(&self, f: F) -> bool
    where
        F: FnOnce(&T) -> bool;
}

impl<T> OptionExt<T> for Option<T> {
    fn is_none_or<F>(&self, f: F) -> bool
    where
        F: FnOnce(&T) -> bool,
    {
        match self {
            None => true,
            Some(ref t) => f(t),
        }
    }
}
