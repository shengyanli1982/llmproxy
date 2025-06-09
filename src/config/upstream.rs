use crate::config::common::BreakerConfig;
use crate::config::defaults::default_uuid_v4_string;
use serde::{Deserialize, Serialize};

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
