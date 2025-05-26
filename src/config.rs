use crate::error::AppError;
use crate::r#const::{http_client_limits, rate_limit_limits, retry_limits, weight_limits};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tracing::debug;
use url::Url;

/// 配置文件结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// HTTP服务器配置
    #[serde(default)]
    pub http_server: HttpServerConfig,
    /// 上游定义
    #[serde(default)]
    pub upstreams: Vec<UpstreamConfig>,
    /// 上游组定义
    #[serde(default)]
    pub upstream_groups: Vec<UpstreamGroupConfig>,
}

impl Config {
    /// 从文件加载配置
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, AppError> {
        let path = path.as_ref();
        debug!("Attempting to load configuration from file: {:?}", path);

        // 打开并读取文件
        let mut file = File::open(path).map_err(|e| {
            AppError::Config(format!(
                "Unable to open configuration file {:?}: {}",
                path, e
            ))
        })?;

        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|e| {
            AppError::Config(format!(
                "Unable to read configuration file {:?}: {}",
                path, e
            ))
        })?;

        // 解析YAML
        let config: Config = serde_yaml::from_str(&content)
            .map_err(|e| AppError::Config(format!("Configuration file parsing error: {}", e)))?;

        // 验证配置
        config.validate()?;

        Ok(config)
    }

    /// 验证配置
    pub fn validate(&self) -> Result<(), AppError> {
        // 验证名称唯一性
        self.validate_name_uniqueness()?;

        // 验证上游组
        let upstream_names: Vec<String> = self.upstreams.iter().map(|u| u.name.clone()).collect();

        // 检查上游组中引用的上游是否存在
        for group in &self.upstream_groups {
            for upstream_ref in &group.upstreams {
                if !upstream_names.contains(&upstream_ref.name) {
                    return Err(AppError::Config(format!(
                        "Upstream group '{}' references non-existent upstream '{}'",
                        group.name, upstream_ref.name
                    )));
                }

                // 验证权重值
                if upstream_ref.weight < weight_limits::MIN_WEIGHT
                    || upstream_ref.weight > weight_limits::MAX_WEIGHT
                {
                    return Err(AppError::Config(format!(
                        "Weight {} for upstream '{}' in group '{}' is out of valid range [{}-{}]",
                        upstream_ref.weight,
                        upstream_ref.name,
                        group.name,
                        weight_limits::MIN_WEIGHT,
                        weight_limits::MAX_WEIGHT
                    )));
                }
            }

            // 确保每个组至少有一个上游
            if group.upstreams.is_empty() {
                return Err(AppError::Config(format!(
                    "Upstream group '{}' has no defined upstreams",
                    group.name
                )));
            }

            // 验证 HTTP 客户端配置
            self.validate_http_client_config(
                &group.http_client,
                &format!("Upstream group '{}'", group.name),
            )?;
        }

        // 检查转发服务引用的上游组是否存在
        let group_names: Vec<String> = self
            .upstream_groups
            .iter()
            .map(|g| g.name.clone())
            .collect();

        for forward in &self.http_server.forwards {
            if !group_names.contains(&forward.upstream_group) {
                return Err(AppError::Config(format!(
                    "Forwarding service '{}' references non-existent upstream group '{}'",
                    forward.name, forward.upstream_group
                )));
            }

            // 验证限流配置
            if forward.ratelimit.enabled {
                if forward.ratelimit.per_second < rate_limit_limits::MIN_PER_SECOND
                    || forward.ratelimit.per_second > rate_limit_limits::MAX_PER_SECOND
                {
                    return Err(AppError::Config(format!(
                        "Requests per second {} for forwarding service '{}' is out of valid range [{}-{}]",
                        forward.ratelimit.per_second,
                        forward.name,
                        rate_limit_limits::MIN_PER_SECOND,
                        rate_limit_limits::MAX_PER_SECOND
                    )));
                }

                if forward.ratelimit.burst < rate_limit_limits::MIN_BURST
                    || forward.ratelimit.burst > rate_limit_limits::MAX_BURST
                {
                    return Err(AppError::Config(format!(
                        "Burst limit {} for forwarding service '{}' is out of valid range [{}-{}]",
                        forward.ratelimit.burst,
                        forward.name,
                        rate_limit_limits::MIN_BURST,
                        rate_limit_limits::MAX_BURST
                    )));
                }
            }

            // 验证超时配置
            self.validate_timeout_config(
                &forward.timeout,
                &format!("Forwarding service '{}'", forward.name),
            )?;
        }

        // 验证管理服务的超时配置
        self.validate_timeout_config(&self.http_server.admin.timeout, "Admin service")?;

        // 验证上游配置
        for upstream in &self.upstreams {
            // 验证 URL 格式
            if let Err(e) = Url::parse(&upstream.url) {
                return Err(AppError::Config(format!(
                    "URL '{}' for upstream '{}' is invalid: {}",
                    upstream.url, upstream.name, e
                )));
            }

            // 验证认证配置
            if let Some(auth) = &upstream.auth {
                match auth.r#type {
                    AuthType::Bearer => {
                        if auth.token.as_ref().map_or(true, |s| s.is_empty()) {
                            return Err(AppError::Config(format!(
                                "Upstream '{}' uses Bearer authentication but no valid token was provided",
                                upstream.name
                            )));
                        }
                    }
                    AuthType::Basic => {
                        if auth.username.as_ref().map_or(true, |s| s.is_empty())
                            || auth.password.as_ref().map_or(true, |s| s.is_empty())
                        {
                            return Err(AppError::Config(format!(
                                "Upstream '{}' uses Basic authentication but no valid username and password were provided",
                                upstream.name
                            )));
                        }
                    }
                    AuthType::None => {}
                }
            }

            // 验证请求头操作
            for header_op in &upstream.headers {
                match header_op.op {
                    HeaderOpType::Insert | HeaderOpType::Replace => {
                        if header_op.value.as_ref().map_or(true, |s| s.is_empty()) {
                            return Err(AppError::Config(format!(
                                "Header operation {:?} for upstream '{}' requires a valid value",
                                header_op.op, upstream.name
                            )));
                        }
                    }
                    HeaderOpType::Remove => {}
                }
            }
        }

        Ok(())
    }

    /// 验证名称唯一性
    fn validate_name_uniqueness(&self) -> Result<(), AppError> {
        // 验证转发服务名称唯一性
        let mut forward_names = HashSet::new();
        for forward in &self.http_server.forwards {
            if !forward_names.insert(&forward.name) {
                return Err(AppError::Config(format!(
                    "Forwarding service name '{}' is duplicated",
                    forward.name
                )));
            }
        }

        // 验证上游名称唯一性
        let mut upstream_names = HashSet::new();
        for upstream in &self.upstreams {
            if !upstream_names.insert(&upstream.name) {
                return Err(AppError::Config(format!(
                    "Upstream name '{}' is duplicated",
                    upstream.name
                )));
            }
        }

        // 验证上游组名称唯一性
        let mut group_names = HashSet::new();
        for group in &self.upstream_groups {
            if !group_names.insert(&group.name) {
                return Err(AppError::Config(format!(
                    "Upstream group name '{}' is duplicated",
                    group.name
                )));
            }
        }

        Ok(())
    }

    /// 验证超时配置
    fn validate_timeout_config(
        &self,
        timeout: &TimeoutConfig,
        context: &str,
    ) -> Result<(), AppError> {
        if timeout.connect < http_client_limits::MIN_CONNECT_TIMEOUT
            || timeout.connect > http_client_limits::MAX_CONNECT_TIMEOUT
        {
            return Err(AppError::Config(format!(
                "Connect timeout {}s for {} is out of valid range [{}-{}]s",
                timeout.connect,
                context,
                http_client_limits::MIN_CONNECT_TIMEOUT,
                http_client_limits::MAX_CONNECT_TIMEOUT
            )));
        }
        Ok(())
    }

    /// 验证 HTTP 客户端配置
    fn validate_http_client_config(
        &self,
        config: &HttpClientConfig,
        context: &str,
    ) -> Result<(), AppError> {
        // 验证连接超时
        if config.timeout.connect < http_client_limits::MIN_CONNECT_TIMEOUT
            || config.timeout.connect > http_client_limits::MAX_CONNECT_TIMEOUT
        {
            return Err(AppError::Config(format!(
                "Connect timeout {}s for {} is out of valid range [{}-{}]s",
                config.timeout.connect,
                context,
                http_client_limits::MIN_CONNECT_TIMEOUT,
                http_client_limits::MAX_CONNECT_TIMEOUT
            )));
        }

        // 验证请求超时
        if config.timeout.request < http_client_limits::MIN_REQUEST_TIMEOUT
            || config.timeout.request > http_client_limits::MAX_REQUEST_TIMEOUT
        {
            return Err(AppError::Config(format!(
                "Request timeout {}s for {} is out of valid range [{}-{}]s",
                config.timeout.request,
                context,
                http_client_limits::MIN_REQUEST_TIMEOUT,
                http_client_limits::MAX_REQUEST_TIMEOUT
            )));
        }

        // 验证空闲连接超时
        if config.timeout.idle < http_client_limits::MIN_IDLE_TIMEOUT
            || config.timeout.idle > http_client_limits::MAX_IDLE_TIMEOUT
        {
            return Err(AppError::Config(format!(
                "Idle connection timeout {}s for {} is out of valid range [{}-{}]s",
                config.timeout.idle,
                context,
                http_client_limits::MIN_IDLE_TIMEOUT,
                http_client_limits::MAX_IDLE_TIMEOUT
            )));
        }

        // 验证TCP Keepalive
        if config.keepalive < http_client_limits::MIN_KEEPALIVE
            || config.keepalive > http_client_limits::MAX_KEEPALIVE
        {
            return Err(AppError::Config(format!(
                "TCP Keepalive {}s for {} is out of valid range [{}-{}]s",
                config.keepalive,
                context,
                http_client_limits::MIN_KEEPALIVE,
                http_client_limits::MAX_KEEPALIVE
            )));
        }

        // 验证重试配置
        if config.retry.enabled {
            if config.retry.attempts < retry_limits::MIN_ATTEMPTS
                || config.retry.attempts > retry_limits::MAX_ATTEMPTS
            {
                return Err(AppError::Config(format!(
                    "Retry attempts {} for {} is out of valid range [{}-{}]",
                    config.retry.attempts,
                    context,
                    retry_limits::MIN_ATTEMPTS,
                    retry_limits::MAX_ATTEMPTS
                )));
            }

            if config.retry.initial < retry_limits::MIN_INITIAL_INTERVAL
                || config.retry.initial > retry_limits::MAX_INITIAL_INTERVAL
            {
                return Err(AppError::Config(format!(
                    "Initial retry interval {}ms for {} is out of valid range [{}-{}]ms",
                    config.retry.initial,
                    context,
                    retry_limits::MIN_INITIAL_INTERVAL,
                    retry_limits::MAX_INITIAL_INTERVAL
                )));
            }
        }

        // 验证代理配置
        if config.proxy.enabled {
            if config.proxy.url.is_empty() {
                return Err(AppError::Config(format!(
                    "Proxy URL for {} cannot be empty when proxy is enabled",
                    context
                )));
            }
            if let Err(e) = Url::parse(&config.proxy.url) {
                return Err(AppError::Config(format!(
                    "Proxy URL '{}' is invalid: {}",
                    config.proxy.url, e
                )));
            }
        }

        Ok(())
    }
}

/// HTTP服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpServerConfig {
    /// 转发服务配置
    #[serde(default)]
    pub forwards: Vec<ForwardConfig>,
    /// 管理服务配置
    #[serde(default)]
    pub admin: AdminConfig,
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            forwards: Vec::new(),
            admin: AdminConfig::default(),
        }
    }
}

/// 转发服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardConfig {
    /// 转发服务名称
    pub name: String,
    /// 监听端口
    pub port: u16,
    /// 监听地址
    #[serde(default = "default_listen_address")]
    pub address: String,
    /// 指向的上游组名
    pub upstream_group: String,
    /// 限流配置
    #[serde(default)]
    pub ratelimit: RateLimitConfig,
    /// 超时配置
    #[serde(default)]
    pub timeout: TimeoutConfig,
}

/// 限流配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// 是否启用限流
    #[serde(default)]
    pub enabled: bool,
    /// 每秒请求数
    #[serde(default = "default_per_second")]
    pub per_second: u32,
    /// 突发请求上限
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

/// 超时配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// 连接超时（秒）
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

/// 管理服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    /// 监听端口
    #[serde(default = "default_admin_port")]
    pub port: u16,
    /// 监听地址
    #[serde(default = "default_listen_address")]
    pub address: String,
    /// 超时配置
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

/// 上游配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamConfig {
    /// 上游名称
    pub name: String,
    /// 上游URL
    pub url: String,
    /// 认证配置
    #[serde(default)]
    pub auth: Option<AuthConfig>,
    /// 请求头操作
    #[serde(default)]
    pub headers: Vec<HeaderOperation>,
}

/// 认证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// 认证类型
    #[serde(default)]
    pub r#type: AuthType,
    /// 认证令牌（用于Bearer认证）
    #[serde(default)]
    pub token: Option<String>,
    /// 用户名（用于Basic认证）
    #[serde(default)]
    pub username: Option<String>,
    /// 密码（用于Basic认证）
    #[serde(default)]
    pub password: Option<String>,
}

/// 认证类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    /// Bearer令牌认证
    Bearer,
    /// 基本认证
    Basic,
    /// 无认证
    None,
}

impl Default for AuthType {
    fn default() -> Self {
        Self::None
    }
}

/// 请求头操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderOperation {
    /// 操作类型
    pub op: HeaderOpType,
    /// 头部名称
    pub key: String,
    /// 头部值（对于insert和replace操作）
    #[serde(default)]
    pub value: Option<String>,
}

/// 请求头操作类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HeaderOpType {
    /// 插入（如果不存在）
    Insert,
    /// 删除
    Remove,
    /// 替换（如果存在）或插入（如果不存在）
    Replace,
}

/// 上游组配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamGroupConfig {
    /// 上游组名称
    pub name: String,
    /// 上游引用列表
    pub upstreams: Vec<UpstreamRef>,
    /// 负载均衡策略
    #[serde(default)]
    pub balance: BalanceConfig,
    /// HTTP客户端配置
    #[serde(default)]
    pub http_client: HttpClientConfig,
}

/// 上游引用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamRef {
    /// 上游名称
    pub name: String,
    /// 权重（用于加权轮询策略）
    #[serde(default = "default_weight")]
    pub weight: u32,
}

/// 负载均衡策略配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceConfig {
    /// 策略类型
    #[serde(default)]
    pub strategy: BalanceStrategy,
}

impl Default for BalanceConfig {
    fn default() -> Self {
        Self {
            strategy: BalanceStrategy::default(),
        }
    }
}

/// 负载均衡策略类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BalanceStrategy {
    /// 轮询
    #[serde(rename = "roundrobin")]
    RoundRobin,
    /// 加权轮询
    #[serde(rename = "weighted_roundrobin")]
    WeightedRoundRobin,
    /// 随机
    Random,
}

impl Default for BalanceStrategy {
    fn default() -> Self {
        Self::RoundRobin
    }
}

/// HTTP客户端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpClientConfig {
    /// 用户代理
    #[serde(default = "default_user_agent")]
    pub agent: String,
    /// TCP Keepalive（秒）
    #[serde(default = "default_keepalive")]
    pub keepalive: u32,
    /// 超时配置
    #[serde(default)]
    pub timeout: HttpClientTimeoutConfig,
    /// 重试配置
    #[serde(default)]
    pub retry: RetryConfig,
    /// 代理配置
    #[serde(default)]
    pub proxy: ProxyConfig,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            agent: default_user_agent(),
            keepalive: default_keepalive(),
            timeout: HttpClientTimeoutConfig::default(),
            retry: RetryConfig::default(),
            proxy: ProxyConfig::default(),
        }
    }
}

/// HTTP客户端超时配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpClientTimeoutConfig {
    /// 连接超时（秒）
    #[serde(default = "default_connect_timeout")]
    pub connect: u64,
    /// 请求超时（秒）
    #[serde(default = "default_request_timeout")]
    pub request: u64,
    /// 空闲超时（秒）
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

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// 是否启用重试
    #[serde(default)]
    pub enabled: bool,
    /// 最大重试次数
    #[serde(default = "default_retry_attempts")]
    pub attempts: u32,
    /// 初始重试间隔（毫秒）
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

/// 代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// 是否启用代理
    #[serde(default)]
    pub enabled: bool,
    /// 代理URL
    #[serde(default)]
    pub url: String,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: String::new(),
        }
    }
}

// 默认值函数
fn default_listen_address() -> String {
    "0.0.0.0".to_string()
}

fn default_admin_port() -> u16 {
    9000
}

fn default_connect_timeout() -> u64 {
    http_client_limits::DEFAULT_CONNECT_TIMEOUT
}

fn default_request_timeout() -> u64 {
    http_client_limits::DEFAULT_REQUEST_TIMEOUT
}

fn default_idle_timeout() -> u64 {
    http_client_limits::DEFAULT_IDLE_TIMEOUT
}

fn default_keepalive() -> u32 {
    http_client_limits::DEFAULT_KEEPALIVE
}

fn default_user_agent() -> String {
    "LLMProxy/1.0".to_string()
}

fn default_retry_attempts() -> u32 {
    retry_limits::DEFAULT_ATTEMPTS
}

fn default_retry_initial() -> u32 {
    retry_limits::DEFAULT_INITIAL_INTERVAL
}

fn default_weight() -> u32 {
    weight_limits::MIN_WEIGHT
}

fn default_per_second() -> u32 {
    rate_limit_limits::DEFAULT_PER_SECOND
}

fn default_burst() -> u32 {
    rate_limit_limits::DEFAULT_BURST
}
