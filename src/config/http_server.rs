use crate::config::common::{RateLimitConfig, TimeoutConfig};
use crate::config::defaults::{default_admin_port, default_listen_address, default_listen_port};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

// HTTP服务器配置
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema, Validate)]
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
    pub upstream_group: String,
    // 限流配置
    #[serde(default)]
    #[validate(nested)]
    pub ratelimit: RateLimitConfig,
    // 超时配置
    #[serde(default)]
    #[validate(nested)]
    pub timeout: TimeoutConfig,
}

// 管理服务配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
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
