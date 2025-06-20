use crate::{
    api::v1::handlers::utils::{
        find_by_name, log_request_body, log_response_body, not_found_error, success_response,
    },
    api::v1::models::{ErrorResponse, SuccessResponse},
    config::{Config, UpstreamConfig},
    r#const::api::error_types,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use validator::Validate;

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

    // 构建响应
    let response = SuccessResponse::success_with_data(upstreams);

    // 记录响应体
    log_response_body(&response);

    Json(response)
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
    let upstream = find_by_name(&config_read.upstreams, &name, |u| &u.name);

    match upstream {
        Some(upstream) => {
            info!("API: Retrieved upstream service '{}'", name);

            // 记录响应体
            let response = SuccessResponse::success_with_data(upstream.clone());
            log_response_body(&response);

            success_response(upstream)
        }
        None => {
            let error = ErrorResponse::error(
                StatusCode::NOT_FOUND,
                error_types::NOT_FOUND,
                format!("Upstream service '{}' does not exist", name),
            );
            log_response_body(&error);
            not_found_error("Upstream service", &name)
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
    // 记录请求体
    log_request_body(&new_upstream);

    // 验证上游服务配置
    if let Err(e) = new_upstream.validate() {
        warn!("API: Upstream validation failed: {}", e);
        let error = ErrorResponse::from_validation_errors(e);
        log_response_body(&error);
        return Json(error).into_response();
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
        let error = ErrorResponse::error(
            StatusCode::CONFLICT,
            error_types::CONFLICT,
            format!("Upstream '{}' already exists", new_upstream.name),
        );
        log_response_body(&error);
        return Json(error).into_response();
    }

    // 添加新的上游服务
    let upstream_clone = new_upstream.clone();
    config_write.upstreams.push(new_upstream);

    // 预处理配置（解析头部等）
    if let Err(e) = config_write.post_process() {
        warn!("API: Failed to process new upstream: {}", e);
        let error = ErrorResponse::error(
            StatusCode::BAD_REQUEST,
            error_types::BAD_REQUEST,
            format!("Failed to process new upstream: {}", e),
        );
        log_response_body(&error);
        return Json(error).into_response();
    }

    info!("API: Created upstream service '{}'", upstream_clone.name);

    // 构建成功响应并记录
    let response = SuccessResponse::success_with_data(upstream_clone);
    log_response_body(&response);

    (StatusCode::CREATED, Json(response)).into_response()
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
    // 记录请求体
    log_request_body(&updated_upstream);

    // 设置名称为路径中的名称
    updated_upstream.name = name.clone();

    // 验证上游服务配置
    if let Err(e) = updated_upstream.validate() {
        warn!("API: Upstream validation failed: {}", e);
        let error = ErrorResponse::from_validation_errors(e);
        log_response_body(&error);
        return Json(error).into_response();
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
                let error = ErrorResponse::error(
                    StatusCode::BAD_REQUEST,
                    error_types::BAD_REQUEST,
                    format!("Failed to process updated upstream: {}", e),
                );
                log_response_body(&error);
                return Json(error).into_response();
            }

            info!("API: Updated upstream service '{}'", name);

            // 构建成功响应并记录
            let response = SuccessResponse::success_with_data(updated_upstream);
            log_response_body(&response);

            Json(response).into_response()
        }
        None => {
            warn!("API: Upstream service '{}' not found for update", name);
            let error = ErrorResponse::error(
                StatusCode::NOT_FOUND,
                error_types::NOT_FOUND,
                format!("Upstream service '{}' does not exist", name),
            );
            log_response_body(&error);
            Json(error).into_response()
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
        let error = ErrorResponse::error(
            StatusCode::CONFLICT,
            error_types::CONFLICT,
            format!(
                "Cannot delete upstream '{}' as it is currently used by group(s): {:?}",
                name, dependent_groups
            ),
        );
        log_response_body(&error);
        return Json(error).into_response();
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

    // 无内容响应，状态码本身就足够
    debug!("Response body: None (204 No Content)");
    StatusCode::NO_CONTENT.into_response()
}
