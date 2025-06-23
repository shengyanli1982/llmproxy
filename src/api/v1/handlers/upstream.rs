use crate::{
    api::v1::handlers::utils::{
        find_by_name, log_request_body, log_response_body, not_found_error, success_response_ref,
    },
    api::v1::models::{ErrorResponse, SuccessResponse},
    api::v1::routes::AppState,
    config::Config,
    config::UpstreamConfig,
    r#const::api::error_types,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use tracing::{debug, info, warn};
use validator::Validate;

// 处理上游服务不存在的错误
#[inline(always)]
fn upstream_not_found(name: &str) -> Response {
    let error = ErrorResponse::error(
        StatusCode::NOT_FOUND,
        error_types::NOT_FOUND,
        format!("Upstream service '{}' does not exist", name),
    );
    log_response_body(&error);
    not_found_error("Upstream service", name)
}

// 处理名称冲突错误
#[inline(always)]
fn name_conflict_error(name: &str) -> Response {
    let error = ErrorResponse::error(
        StatusCode::CONFLICT,
        error_types::CONFLICT,
        format!("Upstream '{}' already exists", name),
    );
    log_response_body(&error);
    Json(error).into_response()
}

// 处理上游服务被使用的错误
#[inline(always)]
fn dependent_groups_error(name: &str, dependent_groups: &[String]) -> Response {
    let error = ErrorResponse::error(
        StatusCode::CONFLICT,
        error_types::CONFLICT,
        format!(
            "Cannot delete upstream '{}' as it is currently used by group(s): {:?}",
            name, dependent_groups
        ),
    );
    log_response_body(&error);
    Json(error).into_response()
}

// 查找上游服务
#[inline(always)]
fn find_upstream<'a>(
    config: &'a impl std::ops::Deref<Target = Config>,
    name: &str,
) -> Option<&'a UpstreamConfig> {
    find_by_name(&config.upstreams, name, |u| &u.name)
}

// 处理配置预处理和可能的错误
#[inline(always)]
fn process_config(config_write: &mut Config, error_message: &str) -> Result<(), Response> {
    if let Err(e) = config_write.post_process() {
        warn!("API: {}: {}", error_message, e);
        let error = ErrorResponse::error(
            StatusCode::BAD_REQUEST,
            error_types::BAD_REQUEST,
            format!("{}: {}", error_message, e),
        );
        log_response_body(&error);
        return Err(Json(error).into_response());
    }
    Ok(())
}

// 查找依赖特定上游服务的组
#[inline(always)]
fn find_dependent_groups(config: &Config, upstream_name: &str) -> Vec<String> {
    config
        .upstream_groups
        .iter()
        .filter(|group| group.upstreams.iter().any(|u| u.name == upstream_name))
        .map(|group| group.name.clone())
        .collect()
}

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
    State(app_state): State<AppState>,
) -> Json<SuccessResponse<Vec<UpstreamConfig>>> {
    let upstreams = app_state.config.read().await.upstreams.clone();
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
pub async fn get_upstream(State(app_state): State<AppState>, Path(name): Path<String>) -> Response {
    // 查找指定名称的上游服务
    let config_read = app_state.config.read().await;
    let upstream = find_upstream(&config_read, &name);

    match upstream {
        Some(upstream) => {
            info!("API: Retrieved upstream service '{}'", name);

            // 记录响应体
            let response = SuccessResponse::success_with_data(upstream);
            log_response_body(&response);

            success_response_ref(upstream)
        }
        None => upstream_not_found(&name),
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
    State(app_state): State<AppState>,
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
    let mut config_write = app_state.config.write().await;

    // 检查名称是否已存在
    if config_write
        .upstreams
        .iter()
        .any(|u| u.name == new_upstream.name)
    {
        warn!("API: Upstream '{}' already exists", new_upstream.name);
        return name_conflict_error(&new_upstream.name);
    }

    // 添加新的上游服务
    let upstream_clone = new_upstream.clone();
    config_write.upstreams.push(new_upstream);

    // 预处理配置（解析头部等）
    if let Err(response) = process_config(&mut config_write, "Failed to process new upstream") {
        return response;
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
    State(app_state): State<AppState>,
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
    let mut config_write = app_state.config.write().await;

    // 查找并更新上游服务
    let upstream_index = config_write.upstreams.iter().position(|u| u.name == name);

    match upstream_index {
        Some(index) => {
            // 更新上游服务
            config_write.upstreams[index] = updated_upstream.clone();

            // 预处理配置（解析头部等）
            if let Err(response) =
                process_config(&mut config_write, "Failed to process updated upstream")
            {
                return response;
            }

            info!("API: Updated upstream service '{}'", name);

            // 构建成功响应并记录
            let response = SuccessResponse::success_with_data(updated_upstream);
            log_response_body(&response);

            Json(response).into_response()
        }
        None => {
            warn!("API: Upstream service '{}' not found for update", name);
            upstream_not_found(&name)
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
    State(app_state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    // 获取写锁
    let mut config_write = app_state.config.write().await;

    // 检查是否被任何上游组使用
    let dependent_groups = find_dependent_groups(&config_write, &name);

    if !dependent_groups.is_empty() {
        warn!(
            "API: Cannot delete upstream '{}' as it is used by groups: {:?}",
            name, dependent_groups
        );
        return dependent_groups_error(&name, &dependent_groups);
    }

    // 查找并删除上游服务
    let upstream_index = config_write.upstreams.iter().position(|u| u.name == name);

    match upstream_index {
        Some(index) => {
            config_write.upstreams.remove(index);
            info!("API: Deleted upstream service '{}'", name);

            debug!("Response body: None (204 No Content)");
            StatusCode::NO_CONTENT.into_response()
        }
        None => {
            warn!("API: Upstream service '{}' not found for deletion", name);
            upstream_not_found(&name)
        }
    }
}
