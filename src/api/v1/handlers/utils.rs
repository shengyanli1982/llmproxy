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
use serde::Serialize;
use serde_json;
use std::collections::HashMap;
use tracing::{debug, warn};
/// 生成"资源未找到"错误响应
pub fn not_found_error(resource_type: &str, name: &str) -> Response {
    warn!("API: {} '{}' not found", resource_type, name);
    Json(ErrorResponse::error(
        StatusCode::NOT_FOUND,
        api::error_types::NOT_FOUND,
        format!("{} '{}' does not exist", resource_type, name),
    ))
    .into_response()
}

/// 生成成功响应，避免不必要的克隆
pub fn success_response<T: Clone + Serialize>(item: &T) -> Response {
    Json(SuccessResponse::success_with_data(item.clone())).into_response()
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
