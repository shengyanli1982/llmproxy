use crate::config::common::{RateLimitConfig, TimeoutConfig};
use crate::config::defaults::{default_admin_port, default_listen_address, default_listen_port};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

// 路由规则
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "lowercase")]
pub struct RoutingRule {
    // 路径模式
    #[validate(length(min = 1, message = "Path pattern cannot be empty"))]
    pub path: String,
    // 目标上游组
    #[validate(length(min = 1, message = "Target group cannot be empty"))]
    pub target_group: String,
}

// HTTP服务器配置
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema, Validate)]
#[serde(rename_all = "lowercase")]
pub struct HttpServerConfig {
    // 转发服务配置
    #[serde(default)]
    #[validate(nested)]
    pub forwards: Vec<ForwardConfig>,
    // 管理服务配置
    #[serde(default)]
    #[validate(nested)]
    pub admin: AdminConfig,
}

// 转发服务配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "lowercase")]
pub struct ForwardConfig {
    // 转发服务名称
    #[validate(length(min = 1, message = "Forward service name cannot be empty"))]
    pub name: String,
    // 监听端口
    #[serde(default = "default_listen_port")]
    pub port: u16,
    // 监听地址
    #[serde(default = "default_listen_address")]
    pub address: String,
    // 指向的上游组名
    #[validate(length(min = 1, message = "Upstream group cannot be empty"))]
    pub default_group: String,
    // 限流配置
    #[serde(default)]
    #[validate(nested)]
    pub ratelimit: Option<RateLimitConfig>,
    // 超时配置
    #[serde(default)]
    #[validate(nested)]
    pub timeout: Option<TimeoutConfig>,
    // 路由规则配置
    #[serde(default)]
    #[validate(nested)]
    pub routing: Option<Vec<RoutingRule>>,
}

// 管理服务配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "lowercase")]
pub struct AdminConfig {
    // 监听端口
    #[serde(default = "default_admin_port")]
    pub port: u16,
    // 监听地址
    #[serde(default = "default_listen_address")]
    pub address: String,
    // 超时配置
    #[serde(default)]
    #[validate(nested)]
    pub timeout: Option<TimeoutConfig>,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            port: default_admin_port(),
            address: default_listen_address(),
            timeout: None,
        }
    }
}
