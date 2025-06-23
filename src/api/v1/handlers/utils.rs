use crate::{
    api::v1::models::{ErrorResponse, SuccessResponse},
    config::UpstreamConfig,
    r#const::api,
};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose, Engine as _};
use serde::Serialize;
use serde_json;
use std::collections::HashMap;
use tracing::{debug, warn};

/// 生成"资源未找到"错误响应
pub fn not_found_error(resource_type: &str, name: &str) -> Response {
    warn!("API: {} '{}' not found", resource_type, name);
    let error = ErrorResponse::error(
        StatusCode::NOT_FOUND,
        api::error_types::NOT_FOUND,
        format!("{} '{}' does not exist", resource_type, name),
    );
    (StatusCode::NOT_FOUND, Json(error)).into_response()
}

/// 生成成功响应，支持引用或所有权传递
pub fn success_response<T: Serialize>(item: T) -> Response {
    Json(SuccessResponse::success_with_data(item)).into_response()
}

// 为引用版本提供一个单独的函数，保持向后兼容
pub fn success_response_ref<T: Clone + Serialize>(item: &T) -> Response {
    Json(SuccessResponse::success_with_data(item)).into_response()
}

/// 创建上游服务名称到配置的引用映射
pub fn create_upstream_map(upstreams: &[UpstreamConfig]) -> HashMap<&str, &UpstreamConfig> {
    upstreams
        .iter()
        .map(|upstream| (upstream.name.as_str(), upstream))
        .collect()
}

/// 按名称查找资源
pub fn find_by_name<'a, T>(
    items: &'a [T],
    name: &str,
    get_name: impl Fn(&T) -> &str,
) -> Option<&'a T> {
    items.iter().find(|item| get_name(item) == name)
}

/// 记录请求体日志
pub fn log_request_body<T: Serialize>(body: &T) {
    match serde_json::to_string(body) {
        Ok(json_str) => {
            debug!("Request body: {:?}", json_str);
        }
        Err(e) => {
            warn!("Request body is not serializable: {}", e);
        }
    }
}

/// 记录响应体日志
pub fn log_response_body<T: Serialize>(body: &T) {
    match serde_json::to_string(body) {
        Ok(json_str) => {
            debug!("Response body: {:?}", json_str);
        }
        Err(e) => {
            warn!("Response body is not serializable: {}", e);
        }
    }
}

/// 将base64字符串解码为路径
///
/// 用于解析API路径参数，返回Result表示解码可能失败
pub fn decode_base64_to_path(encoded: &str) -> Result<String, String> {
    match general_purpose::URL_SAFE.decode(encoded) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(path) => Ok(path),
            Err(e) => Err(format!("Failed to convert decoded bytes to string: {}", e)),
        },
        Err(e) => Err(format!("Failed to decode base64 string: {}", e)),
    }
}
