use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::fmt;

// API 错误码常量
pub mod codes {
    // 成功
    pub const OK: u32 = 20000;
    pub const CREATED: u32 = 20100;
    pub const ACCEPTED: u32 = 20200;

    // 客户端错误
    pub const BAD_REQUEST: u32 = 40000;
    pub const INVALID_JSON: u32 = 40001;
    pub const MISSING_FIELD: u32 = 40002;
    pub const UNAUTHORIZED: u32 = 40100;
    pub const FORBIDDEN: u32 = 40300;
    pub const NOT_FOUND: u32 = 40400;
    pub const CONFLICT: u32 = 40900;
    pub const ALREADY_EXISTS: u32 = 40901;
    pub const STILL_REFERENCED: u32 = 40902;
    pub const UNPROCESSABLE: u32 = 42200;

    // 服务器错误
    pub const INTERNAL_ERROR: u32 = 50000;
    pub const SERVICE_UNAVAILABLE: u32 = 50300;
}

// API 错误类型
#[derive(Debug)]
pub enum ApiError {
    // 资源不存在
    NotFound(String),
    // 资源已存在
    AlreadyExists(String),
    // 资源仍被引用
    StillReferenced {
        resource_type: String,
        name: String,
        referenced_by: Vec<String>,
    },
    // 引用资源不存在
    ReferenceNotFound {
        resource_type: String,
        name: String,
    },
    // 参数验证失败
    ValidationError(String),
    // 内部错误
    InternalError(String),
    // JSON解析错误
    JsonParseError(String),
    // 参数缺失
    MissingParameter(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "Resource not found: {}", msg),
            Self::AlreadyExists(msg) => write!(f, "Resource already exists: {}", msg),
            Self::StillReferenced {
                resource_type,
                name,
                referenced_by,
            } => {
                write!(
                    f,
                    "{} '{}' is still referenced by: {:?}",
                    resource_type, name, referenced_by
                )
            }
            Self::ReferenceNotFound {
                resource_type,
                name,
            } => {
                write!(f, "Referenced {} '{}' not found", resource_type, name)
            }
            Self::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            Self::InternalError(msg) => write!(f, "Internal error: {}", msg),
            Self::JsonParseError(msg) => write!(f, "JSON parse error: {}", msg),
            Self::MissingParameter(msg) => write!(f, "Missing parameter: {}", msg),
        }
    }
}

// API 响应结构
#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub code: u32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    // 创建成功响应
    pub fn ok(message: impl Into<String>, data: Option<T>) -> Self {
        Self {
            code: codes::OK,
            message: message.into(),
            data,
        }
    }

    // 创建资源创建成功响应
    pub fn created(message: impl Into<String>, data: Option<T>) -> Self {
        Self {
            code: codes::CREATED,
            message: message.into(),
            data,
        }
    }

    // 创建请求已接受响应（异步处理）
    pub fn accepted(message: impl Into<String>, data: Option<T>) -> Self {
        Self {
            code: codes::ACCEPTED,
            message: message.into(),
            data,
        }
    }
}

// 实现 API 错误的 HTTP 响应转换
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            Self::NotFound(_) => (StatusCode::NOT_FOUND, codes::NOT_FOUND, self.to_string()),
            Self::AlreadyExists(_) => (
                StatusCode::CONFLICT,
                codes::ALREADY_EXISTS,
                self.to_string(),
            ),
            Self::StillReferenced { .. } => (
                StatusCode::CONFLICT,
                codes::STILL_REFERENCED,
                self.to_string(),
            ),
            Self::ReferenceNotFound { .. } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                codes::UNPROCESSABLE,
                self.to_string(),
            ),
            Self::ValidationError(_) => (
                StatusCode::BAD_REQUEST,
                codes::BAD_REQUEST,
                self.to_string(),
            ),
            Self::InternalError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                codes::INTERNAL_ERROR,
                self.to_string(),
            ),
            Self::JsonParseError(_) => (
                StatusCode::BAD_REQUEST,
                codes::INVALID_JSON,
                self.to_string(),
            ),
            Self::MissingParameter(_) => (
                StatusCode::BAD_REQUEST,
                codes::MISSING_FIELD,
                self.to_string(),
            ),
        };

        // 创建带有额外数据的错误响应
        let response = match &self {
            Self::StillReferenced {
                resource_type: _,
                name: _,
                referenced_by,
            } => ApiResponse {
                code,
                message,
                data: Some(serde_json::json!({
                    "referenced_by": referenced_by
                })),
            },
            _ => ApiResponse {
                code,
                message,
                data: None,
            },
        };

        (status, Json(response)).into_response()
    }
}
