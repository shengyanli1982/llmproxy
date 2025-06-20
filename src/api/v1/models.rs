use crate::{
    config::{UpstreamConfig, UpstreamGroupConfig},
    r#const::api::{error_types, response_status},
};
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::ValidationErrors;

/// 错误详情结构
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorDetail {
    /// 错误类型
    pub r#type: String,
    /// 错误消息
    pub message: String,
}

/// API 统一响应结构
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SuccessResponse<T> {
    /// HTTP 状态码
    pub code: u16,
    /// 响应状态 (始终为 "success")
    pub status: String,
    /// 响应数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

/// 错误响应结构
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    /// HTTP 状态码
    pub code: u16,
    /// 响应状态 (始终为 "error")
    pub status: String,
    /// 错误详情
    pub error: ErrorDetail,
}

/// 上游组详情模型 (扩展了标准的 UpstreamGroupConfig)
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UpstreamGroupDetail {
    /// 上游组名称
    pub name: String,
    /// 上游服务完整配置列表 (而非仅引用)
    pub upstreams: Vec<UpstreamConfig>,
    /// 负载均衡配置
    pub balance: crate::config::BalanceConfig,
    /// HTTP 客户端配置
    pub http_client: crate::config::HttpClientConfig,
}

/// 上游组PATCH操作的请求体
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct PatchUpstreamGroupPayload {
    /// 上游服务引用列表
    pub upstreams: Vec<UpstreamRef>,
}

/// 上游服务引用
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UpstreamRef {
    /// 上游服务名称
    pub name: String,
    /// 权重
    pub weight: u32,
}

impl SuccessResponse<()> {
    /// 创建一个成功响应，无数据
    pub fn success() -> Self {
        Self {
            code: StatusCode::OK.as_u16(),
            status: response_status::SUCCESS.to_string(),
            data: None,
        }
    }
}

impl<T> SuccessResponse<T> {
    /// 创建一个成功响应，带数据
    pub fn success_with_data(data: T) -> Self {
        Self {
            code: StatusCode::OK.as_u16(),
            status: response_status::SUCCESS.to_string(),
            data: Some(data),
        }
    }
}

impl<T: Serialize> IntoResponse for SuccessResponse<T> {
    fn into_response(self) -> axum::response::Response {
        let status_code =
            StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status_code, Json(self)).into_response()
    }
}

impl ErrorResponse {
    /// 创建一个错误响应
    pub fn error(
        status_code: StatusCode,
        error_type: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: status_code.as_u16(),
            status: response_status::ERROR.to_string(),
            error: ErrorDetail {
                r#type: error_type.into(),
                message: message.into(),
            },
        }
    }

    /// 从验证错误创建响应
    pub fn from_validation_errors(errors: ValidationErrors) -> Self {
        // 将所有验证错误平铺为一条消息
        let message = errors
            .field_errors()
            .into_iter()
            .map(|(field, errors)| {
                let messages: Vec<String> = errors
                    .iter()
                    .map(|e| {
                        e.message
                            .as_ref()
                            .map(|m| m.to_string())
                            .unwrap_or_default()
                    })
                    .collect();
                format!("{}: {}", field, messages.join(", "))
            })
            .collect::<Vec<String>>()
            .join("; ");

        Self::error(StatusCode::BAD_REQUEST, error_types::BAD_REQUEST, message)
    }
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> axum::response::Response {
        let status_code =
            StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status_code, Json(self)).into_response()
    }
}

/// 将 UpstreamGroupConfig 转换为 UpstreamGroupDetail
impl UpstreamGroupDetail {
    /// 创建一个新的上游组详情，需要提供上游组配置和所有上游配置的映射
    pub fn from_config(
        group: &UpstreamGroupConfig,
        upstream_map: &std::collections::HashMap<&str, &UpstreamConfig>,
    ) -> Self {
        let upstreams = group
            .upstreams
            .iter()
            .filter_map(|upstream_ref| {
                upstream_map
                    .get(&upstream_ref.name.as_str())
                    .map(|&u| u.clone())
            })
            .collect();

        Self {
            name: group.name.clone(),
            upstreams,
            balance: group.balance.clone(),
            http_client: group.http_client.clone(),
        }
    }
}
