use crate::config::common::BreakerConfig;
use crate::config::defaults::default_weight;
use crate::config::serializer::arc_string;
use crate::config::validation;
use reqwest::header::{HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use validator::Validate;

use super::http_client::HttpClientConfig;

/// 上游服务配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpstreamConfig {
    // 上游服务名称
    #[validate(length(min = 1, message = "Upstream name cannot be empty"))]
    pub name: String,
    // 上游服务地址
    #[serde(with = "arc_string")]
    #[schema(value_type = String)]
    #[validate(url(message = "Upstream URL is invalid"))]
    #[validate(skip)]
    pub url: Arc<String>,
    // 权重
    #[validate(range(min = 1, max = 65535, message = "Weight must be between 1 and 65535"))]
    #[serde(default = "default_weight")]
    pub weight: u32,
    // 认证配置
    #[serde(default)]
    #[validate(nested)]
    pub auth: Option<AuthConfig>,
    // HTTP 客户端配置
    #[serde(default)]
    #[validate(nested)]
    pub http_client: HttpClientConfig,
    // 请求头操作
    #[serde(default)]
    #[validate(nested)]
    pub headers: Vec<HeaderOp>,
    // 熔断器配置
    #[serde(default)]
    #[validate(nested)]
    pub breaker: Option<BreakerConfig>,
}

// 认证配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[validate(schema(function = "validation::validate_auth_config"))]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
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

/// HTTP 请求头操作类型
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, ToSchema)]
pub enum HeaderOpType {
    // 插入
    Insert,
    // 替换
    Replace,
    // 移除
    Remove,
}

// 请求头操作
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[validate(schema(function = "validation::validate_header_op"))]
pub struct HeaderOp {
    pub op: HeaderOpType,
    #[validate(length(min = 1, message = "Header key cannot be empty"))]
    pub key: String,
    pub value: Option<String>,
    #[serde(skip)]
    pub parsed_name: Option<HeaderName>,
    #[serde(skip)]
    pub parsed_value: Option<HeaderValue>,
}
