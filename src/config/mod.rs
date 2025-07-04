pub mod common;
pub mod defaults;
pub mod http_client;
pub mod http_server;
pub mod serializer;
pub mod upstream;
pub mod upstream_group;
pub mod validation;

use crate::error::AppError;
pub use common::{BreakerConfig, ProxyConfig, RateLimitConfig, RetryConfig, TimeoutConfig};
pub use http_client::{HttpClientConfig, HttpClientTimeoutConfig};
pub use http_server::{AdminConfig, ForwardConfig, HttpServerConfig};
use reqwest::header::{HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tracing::debug;
pub use upstream::{AuthConfig, AuthType, HeaderOp, HeaderOpType, UpstreamConfig};
pub use upstream_group::{BalanceConfig, BalanceStrategy, UpstreamGroupConfig, UpstreamRef};
use utoipa::ToSchema;
use validator::Validate;

// 配置文件结构
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[validate(schema(function = "validation::validate_config"))]
pub struct Config {
    // HTTP服务器配置
    #[serde(default)]
    #[validate(nested)]
    pub http_server: Option<HttpServerConfig>,
    // 上游定义
    #[serde(default)]
    #[validate(nested)]
    pub upstreams: Vec<UpstreamConfig>,
    // 上游组定义
    #[serde(default)]
    #[validate(nested)]
    pub upstream_groups: Vec<UpstreamGroupConfig>,
}

impl Config {
    // 从文件加载配置
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
        let mut config: Config = serde_yaml::from_str(&content)
            .map_err(|e| AppError::Config(format!("Configuration file parsing error: {}", e)))?;

        // 预处理配置
        config.post_process()?;

        // 验证配置
        config
            .validate()
            .map_err(|e| AppError::Config(format!("Configuration validation error: {}", e)))?;

        Ok(config)
    }

    // 预处理配置，例如预解析头部
    pub fn post_process(&mut self) -> Result<(), AppError> {
        for upstream in &mut self.upstreams {
            for op in &mut upstream.headers {
                // 预解析头部名称
                let name = HeaderName::from_bytes(op.key.as_bytes()).map_err(|e| {
                    AppError::InvalidHeader(format!(
                        "Invalid header name '{}' for upstream '{}': {}",
                        op.key, upstream.name, e
                    ))
                })?;
                op.parsed_name = Some(name);

                // 预解析头部值
                if let Some(value_str) = &op.value {
                    let value = HeaderValue::from_str(value_str).map_err(|e| {
                        AppError::InvalidHeader(format!(
                            "Invalid header value for key '{}' in upstream '{}': {}",
                            op.key, upstream.name, e
                        ))
                    })?;
                    op.parsed_value = Some(value);
                }
            }
        }
        Ok(())
    }
}
