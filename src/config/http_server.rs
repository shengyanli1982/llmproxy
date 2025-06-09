use crate::config::common::{RateLimitConfig, TimeoutConfig};
use crate::config::defaults::{default_admin_port, default_listen_address, default_listen_port};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// HTTP服务器配置
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct HttpServerConfig {
    // 转发服务配置
    #[serde(default)]
    pub forwards: Vec<ForwardConfig>,
    // 管理服务配置
    #[serde(default)]
    pub admin: AdminConfig,
}

// 转发服务配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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

// 管理服务配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
