use crate::{
    api::v1::models::{ErrorResponse, SuccessResponse},
    config::{Config, UpstreamConfig},
    r#const::api,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// 获取所有上游服务列表
///
/// Get all upstream services list
#[utoipa::path(
    get,
    path = "/api/v1/upstreams",
    tag = "Upstreams",
    responses(
        (status = 200, description = "成功获取所有上游服务 | Successfully retrieved all upstream services", body = SuccessResponse<Vec<UpstreamConfig>>),
        (status = 500, description = "服务器内部错误 | Internal server error", body = ErrorResponse),
    )
)]
pub async fn list_upstreams(
    State(config): State<Arc<RwLock<Config>>>,
) -> Json<SuccessResponse<Vec<UpstreamConfig>>> {
    let upstreams = config.read().await.upstreams.clone();
    info!("API: Retrieved {} upstream services", upstreams.len());
    Json(SuccessResponse::success_with_data(upstreams))
}

/// 获取单个上游服务详情
///
/// Get a single upstream service detail
#[utoipa::path(
    get,
    path = "/api/v1/upstreams/{name}",
    tag = "Upstreams",
    params(
        ("name" = String, Path, description = "上游服务名称 | Upstream service name")
    ),
    responses(
        (status = 200, description = "成功获取上游服务 | Successfully retrieved upstream service", body = SuccessResponse<UpstreamConfig>),
        (status = 404, description = "上游服务不存在 | Upstream service not found", body = ErrorResponse),
        (status = 500, description = "服务器内部错误 | Internal server error", body = ErrorResponse),
    )
)]
#[axum::debug_handler]
pub async fn get_upstream(
    State(config): State<Arc<RwLock<Config>>>,
    Path(name): Path<String>,
) -> Response {
    // 查找指定名称的上游服务
    let config_read = config.read().await;
    match config_read
        .upstreams
        .iter()
        .find(|upstream| upstream.name == name)
    {
        Some(upstream) => {
            info!("API: Retrieved upstream service '{}'", name);
            Json(SuccessResponse::success_with_data(upstream.clone())).into_response()
        }
        None => {
            warn!("API: Upstream service '{}' not found", name);
            Json(ErrorResponse::error(
                StatusCode::NOT_FOUND,
                api::error_types::NOT_FOUND,
                format!("Upstream service '{}' does not exist", name),
            ))
            .into_response()
        }
    }
}

/// 创建新的上游服务
///
/// Create a new upstream service
#[utoipa::path(
    post,
    path = "/api/v1/upstreams",
    tag = "Upstreams",
    request_body = UpstreamConfig,
    responses(
        (status = 201, description = "成功创建上游服务 | Successfully created upstream service", body = SuccessResponse<UpstreamConfig>),
        (status = 400, description = "请求体格式错误或验证失败 | Invalid request body or validation failed", body = ErrorResponse),
        (status = 409, description = "上游服务名称已存在 | Upstream service name already exists", body = ErrorResponse),
        (status = 500, description = "服务器内部错误 | Internal server error", body = ErrorResponse),
    )
)]
pub async fn create_upstream(
    State(config): State<Arc<RwLock<Config>>>,
    Json(new_upstream): Json<UpstreamConfig>,
) -> Response {
    // 验证上游服务配置
    if let Err(e) = new_upstream.validate() {
        warn!("API: Upstream validation failed: {}", e);
        return Json(ErrorResponse::error(
            StatusCode::BAD_REQUEST,
            "validation_error",
            format!("Validation error: {}", e),
        ))
        .into_response();
    }

    // 获取写锁
    let mut config_write = config.write().await;

    // 检查名称是否已存在
    if config_write
        .upstreams
        .iter()
        .any(|u| u.name == new_upstream.name)
    {
        warn!("API: Upstream '{}' already exists", new_upstream.name);
        return Json(ErrorResponse::error(
            StatusCode::CONFLICT,
            "conflict",
            format!("Upstream '{}' already exists", new_upstream.name),
        ))
        .into_response();
    }

    // 添加新的上游服务
    let upstream_clone = new_upstream.clone();
    config_write.upstreams.push(new_upstream);

    // 预处理配置（解析头部等）
    if let Err(e) = config_write.post_process() {
        warn!("API: Failed to process new upstream: {}", e);
        return Json(ErrorResponse::error(
            StatusCode::BAD_REQUEST,
            "processing_error",
            format!("Failed to process upstream: {}", e),
        ))
        .into_response();
    }

    info!("API: Created upstream service '{}'", upstream_clone.name);
    (
        StatusCode::CREATED,
        Json(SuccessResponse::success_with_data(upstream_clone)),
    )
        .into_response()
}

/// 更新上游服务
///
/// Update an existing upstream service
#[utoipa::path(
    put,
    path = "/api/v1/upstreams/{name}",
    tag = "Upstreams",
    params(
        ("name" = String, Path, description = "上游服务名称 | Upstream service name")
    ),
    request_body = UpstreamConfig,
    responses(
        (status = 200, description = "成功更新上游服务 | Successfully updated upstream service", body = SuccessResponse<UpstreamConfig>),
        (status = 400, description = "请求体格式错误或验证失败 | Invalid request body or validation failed", body = ErrorResponse),
        (status = 404, description = "上游服务不存在 | Upstream service not found", body = ErrorResponse),
        (status = 500, description = "服务器内部错误 | Internal server error", body = ErrorResponse),
    )
)]
pub async fn update_upstream(
    State(config): State<Arc<RwLock<Config>>>,
    Path(name): Path<String>,
    Json(mut updated_upstream): Json<UpstreamConfig>,
) -> Response {
    // 设置名称为路径中的名称
    updated_upstream.name = name.clone();

    // 验证上游服务配置
    if let Err(e) = updated_upstream.validate() {
        warn!("API: Upstream validation failed: {}", e);
        return Json(ErrorResponse::error(
            StatusCode::BAD_REQUEST,
            "validation_error",
            format!("Validation error: {}", e),
        ))
        .into_response();
    }

    // 获取写锁
    let mut config_write = config.write().await;

    // 查找并更新上游服务
    let upstream_index = config_write.upstreams.iter().position(|u| u.name == name);

    match upstream_index {
        Some(index) => {
            // 更新上游服务
            config_write.upstreams[index] = updated_upstream.clone();

            // 预处理配置（解析头部等）
            if let Err(e) = config_write.post_process() {
                warn!("API: Failed to process updated upstream: {}", e);
                return Json(ErrorResponse::error(
                    StatusCode::BAD_REQUEST,
                    "processing_error",
                    format!("Failed to process upstream: {}", e),
                ))
                .into_response();
            }

            info!("API: Updated upstream service '{}'", name);
            Json(SuccessResponse::success_with_data(updated_upstream)).into_response()
        }
        None => {
            warn!("API: Upstream service '{}' not found for update", name);
            Json(ErrorResponse::error(
                StatusCode::NOT_FOUND,
                api::error_types::NOT_FOUND,
                format!("Upstream service '{}' does not exist", name),
            ))
            .into_response()
        }
    }
}

/// 删除上游服务
///
/// Delete an upstream service
#[utoipa::path(
    delete,
    path = "/api/v1/upstreams/{name}",
    tag = "Upstreams",
    params(
        ("name" = String, Path, description = "上游服务名称 | Upstream service name")
    ),
    responses(
        (status = 204, description = "成功删除上游服务 | Successfully deleted upstream service"),
        (status = 409, description = "上游服务正在被使用 | Upstream service is in use", body = ErrorResponse),
        (status = 500, description = "服务器内部错误 | Internal server error", body = ErrorResponse),
    )
)]
pub async fn delete_upstream(
    State(config): State<Arc<RwLock<Config>>>,
    Path(name): Path<String>,
) -> Response {
    // 获取写锁
    let mut config_write = config.write().await;

    // 检查是否被任何上游组使用
    let dependent_groups: Vec<String> = config_write
        .upstream_groups
        .iter()
        .filter(|group| group.upstreams.iter().any(|u| u.name == name))
        .map(|group| group.name.clone())
        .collect();

    if !dependent_groups.is_empty() {
        warn!(
            "API: Cannot delete upstream '{}' as it is used by groups: {:?}",
            name, dependent_groups
        );
        return Json(ErrorResponse::error(
            StatusCode::CONFLICT,
            "dependency_conflict",
            format!(
                "Cannot delete upstream '{}' as it is currently used by group(s): {:?}",
                name, dependent_groups
            ),
        ))
        .into_response();
    }

    // 查找并删除上游服务
    let upstream_index = config_write.upstreams.iter().position(|u| u.name == name);

    if let Some(index) = upstream_index {
        config_write.upstreams.remove(index);
        info!("API: Deleted upstream service '{}'", name);
    } else {
        info!(
            "API: Upstream service '{}' not found for deletion, already gone",
            name
        );
    }

    // 无内容响应
    StatusCode::NO_CONTENT.into_response()
}
