use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;

/// API成功响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    /// 状态码，与HTTP状态码保持一致
    pub code: u16,
    /// 状态，始终为"success"
    pub status: String,
    /// 人类可读的成功信息
    pub message: String,
    /// 响应数据
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    /// 创建一个新的成功响应
    pub fn new(data: Option<T>) -> Self {
        Self {
            code: StatusCode::OK.as_u16(),
            status: "success".to_string(),
            message: "Request successful".to_string(),
            data,
        }
    }

    /// 创建一个带有自定义消息的成功响应
    pub fn with_message(data: Option<T>, message: impl Into<String>) -> Self {
        Self {
            code: StatusCode::OK.as_u16(),
            status: "success".to_string(),
            message: message.into(),
            data,
        }
    }

    /// 创建一个带有自定义状态码和消息的成功响应
    pub fn with_code_and_message(code: u16, data: Option<T>, message: impl Into<String>) -> Self {
        Self {
            code,
            status: "success".to_string(),
            message: message.into(),
            data,
        }
    }
}

/// 错误类型
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    /// 验证错误
    ValidationError,
    /// 资源未找到
    ResourceNotFound,
    /// 资源冲突
    ResourceConflict,
    /// 服务器内部错误
    InternalServerError,
}

impl fmt::Display for ErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorType::ValidationError => write!(f, "validation_error"),
            ErrorType::ResourceNotFound => write!(f, "resource_not_found"),
            ErrorType::ResourceConflict => write!(f, "resource_conflict"),
            ErrorType::InternalServerError => write!(f, "internal_server_error"),
        }
    }
}

/// 错误详情
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorDetail {
    /// 资源类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    /// 字段名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// 问题描述
    pub issue: String,
}

/// API错误响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    /// 状态码，与HTTP状态码保持一致
    pub code: u16,
    /// 状态，始终为"error"
    pub status: String,
    /// 错误详情
    pub error: ErrorInfo,
}

/// 错误信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorInfo {
    /// 错误类型
    #[serde(rename = "type")]
    pub error_type: String,
    /// 错误消息
    pub message: String,
    /// 详细错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<ErrorDetail>>,
}

impl ApiError {
    /// 创建一个验证错误响应
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self {
            code: StatusCode::BAD_REQUEST.as_u16(),
            status: "error".to_string(),
            error: ErrorInfo {
                error_type: ErrorType::ValidationError.to_string(),
                message: message.into(),
                details: None,
            },
        }
    }

    /// 创建一个带有详细信息的验证错误响应
    pub fn validation_error_with_details(
        message: impl Into<String>,
        details: Vec<ErrorDetail>,
    ) -> Self {
        Self {
            code: StatusCode::BAD_REQUEST.as_u16(),
            status: "error".to_string(),
            error: ErrorInfo {
                error_type: ErrorType::ValidationError.to_string(),
                message: message.into(),
                details: Some(details),
            },
        }
    }

    /// 创建一个资源未找到错误响应
    pub fn resource_not_found(resource_type: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            code: StatusCode::NOT_FOUND.as_u16(),
            status: "error".to_string(),
            error: ErrorInfo {
                error_type: ErrorType::ResourceNotFound.to_string(),
                message: format!("{} not found: {}", resource_type.into(), name.into()),
                details: None,
            },
        }
    }

    /// 创建一个资源冲突错误响应
    pub fn resource_conflict(message: impl Into<String>) -> Self {
        Self {
            code: StatusCode::CONFLICT.as_u16(),
            status: "error".to_string(),
            error: ErrorInfo {
                error_type: ErrorType::ResourceConflict.to_string(),
                message: message.into(),
                details: None,
            },
        }
    }

    /// 创建一个内部服务器错误响应
    pub fn internal_server_error(message: impl Into<String>) -> Self {
        Self {
            code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            status: "error".to_string(),
            error: ErrorInfo {
                error_type: ErrorType::InternalServerError.to_string(),
                message: message.into(),
                details: None,
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.code {
            code if code == StatusCode::BAD_REQUEST.as_u16() => StatusCode::BAD_REQUEST,
            code if code == StatusCode::NOT_FOUND.as_u16() => StatusCode::NOT_FOUND,
            code if code == StatusCode::CONFLICT.as_u16() => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(self)).into_response()
    }
}

/// 为任何可序列化类型实现IntoResponse
impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let status = match self.code {
            code if code == StatusCode::CREATED.as_u16() => StatusCode::CREATED,
            _ => StatusCode::OK,
        };

        (status, Json(self)).into_response()
    }
}
