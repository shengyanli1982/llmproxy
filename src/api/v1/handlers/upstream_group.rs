use crate::{
    api::v1::handlers::utils::{
        create_upstream_map, find_by_name, log_request_body, log_response_body, not_found_error,
        success_response,
    },
    api::v1::models::{ErrorResponse, SuccessResponse, UpstreamGroupDetail},
    api::v1::routes::AppState,
    config::{validation::check_duplicate_upstreams, UpstreamRef},
    r#const::api::error_types,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::{info, warn};
use utoipa::ToSchema;
use validator::Validate;

/// 获取所有上游组列表
///
/// Get all upstream groups list
#[utoipa::path(
    get,
    path = "/api/v1/upstream-groups",
    tag = "UpstreamGroups",
    responses(
        (status = 200, description = "成功获取所有上游组 | Successfully retrieved all upstream groups", body = SuccessResponse<Vec<UpstreamGroupDetail>>),
        (status = 500, description = "服务器内部错误 | Internal server error", body = ErrorResponse),
    )
)]
pub async fn list_upstream_groups(
    State(app_state): State<AppState>,
) -> Json<SuccessResponse<Vec<UpstreamGroupDetail>>> {
    // 获取读锁
    let config_read = app_state.config.read().await;

    // 创建上游服务名称到配置的映射 (使用引用)
    let upstream_map = create_upstream_map(&config_read.upstreams);

    // 将每个上游组转换为详情模型
    let groups: Vec<UpstreamGroupDetail> = config_read
        .upstream_groups
        .iter()
        .map(|group| UpstreamGroupDetail::from_config(group, &upstream_map))
        .collect();

    info!("API: Retrieved {} upstream groups", groups.len());

    // 构建响应
    let response = SuccessResponse::success_with_data(groups);

    // 记录响应体
    log_response_body(&response);

    Json(response)
}

/// 获取单个上游组详情
///
/// Get a single upstream group detail
#[utoipa::path(
    get,
    path = "/api/v1/upstream-groups/{name}",
    tag = "UpstreamGroups",
    params(
        ("name" = String, Path, description = "上游组名称 | Upstream group name")
    ),
    responses(
        (status = 200, description = "成功获取上游组 | Successfully retrieved upstream group", body = SuccessResponse<UpstreamGroupDetail>),
        (status = 404, description = "上游组不存在 | Upstream group not found", body = ErrorResponse),
        (status = 500, description = "服务器内部错误 | Internal server error", body = ErrorResponse),
    )
)]
#[axum::debug_handler]
pub async fn get_upstream_group(
    State(app_state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    // 获取读锁
    let config_read = app_state.config.read().await;

    // 查找指定名称的上游组
    let group = find_by_name(&config_read.upstream_groups, &name, |g| &g.name);

    match group {
        Some(group) => {
            // 创建上游服务名称到配置的映射
            let upstream_map = create_upstream_map(&config_read.upstreams);

            // 转换为详情模型
            let detail = UpstreamGroupDetail::from_config(group, &upstream_map);
            info!("API: Retrieved upstream group '{}'", name);

            // 记录响应体
            let response = SuccessResponse::success_with_data(&detail);
            log_response_body(&response);

            // 直接返回detail的所有权，避免克隆
            success_response(detail)
        }
        None => {
            let error = ErrorResponse::error(
                StatusCode::NOT_FOUND,
                error_types::NOT_FOUND,
                format!("Upstream group '{}' does not exist", name),
            );
            log_response_body(&error);
            not_found_error("Upstream group", &name)
        }
    }
}

/// 上游组PATCH操作的请求体
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, Validate)]
pub struct RequestPatchUpstreamGroupPayload {
    /// 上游服务引用列表
    #[validate(length(min = 1), nested)]
    pub upstreams: Vec<UpstreamRef>,
}

/// 部分更新上游组
///
/// Partially update an upstream group
#[utoipa::path(
    patch,
    path = "/api/v1/upstream-groups/{name}",
    tag = "UpstreamGroups",
    params(
        ("name" = String, Path, description = "上游组名称 | Upstream group name")
    ),
    request_body = RequestPatchUpstreamGroupPayload,
    responses(
        (status = 200, description = "成功更新上游组 | Successfully updated upstream group", body = SuccessResponse<UpstreamGroupDetail>),
        (status = 400, description = "请求体格式错误或验证失败 | Invalid request body or validation failed", body = ErrorResponse),
        (status = 404, description = "上游组不存在 | Upstream group not found", body = ErrorResponse),
        (status = 500, description = "服务器内部错误 | Internal server error", body = ErrorResponse),
    )
)]
pub async fn patch_upstream_group(
    State(app_state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<RequestPatchUpstreamGroupPayload>,
) -> Response {
    // 记录请求体
    log_request_body(&payload);

    // 验证请求体
    if let Err(e) = payload.validate() {
        warn!("API: Invalid PATCH request for group '{}': {}", name, e);
        let error = ErrorResponse::from_validation_errors(e);
        log_response_body(&error);
        return (StatusCode::BAD_REQUEST, Json(error)).into_response();
    }

    // 检查重复的上游引用
    if let Err(e) = check_duplicate_upstreams(&payload.upstreams, &name) {
        warn!(
            "API: Duplicate upstream references found in request for group '{}': {}",
            name, e
        );
        let error = ErrorResponse::error(
            StatusCode::BAD_REQUEST,
            error_types::BAD_REQUEST,
            e.message
                .unwrap_or_else(|| "Duplicate upstream references found".into())
                .to_string(),
        );
        log_response_body(&error);
        return (StatusCode::BAD_REQUEST, Json(error)).into_response();
    }

    // 获取写锁
    let mut config_write = app_state.config.write().await;

    // 查找上游组索引
    let group_index = config_write
        .upstream_groups
        .iter()
        .position(|group| group.name == name);

    match group_index {
        Some(index) => {
            // 验证所有引用的上游服务是否存在
            let upstream_names: HashSet<&str> = config_write
                .upstreams
                .iter()
                .map(|u| u.name.as_str())
                .collect();

            for upstream_ref in &payload.upstreams {
                if !upstream_names.contains(upstream_ref.name.as_str()) {
                    warn!(
                        "API: Referenced upstream '{}' not found for group '{}'",
                        upstream_ref.name, name
                    );
                    let error = ErrorResponse::error(
                        StatusCode::BAD_REQUEST,
                        error_types::BAD_REQUEST,
                        format!("Upstream '{}' not found", upstream_ref.name),
                    );
                    log_response_body(&error);
                    return (StatusCode::BAD_REQUEST, Json(error)).into_response();
                }
            }

            // 更新上游组的上游列表，直接赋值payload中的上游列表，避免不必要的clone
            config_write.upstream_groups[index].upstreams = payload.upstreams;

            // 创建上游服务名称到配置的映射
            let upstream_map = create_upstream_map(&config_write.upstreams);

            // 创建响应详情
            let detail = UpstreamGroupDetail::from_config(
                &config_write.upstream_groups[index],
                &upstream_map,
            );

            // 更新运行时的负载均衡器
            // 注意：在释放config_write锁后执行，以避免可能的死锁
            let group_name = name.clone();

            // 获取上游引用的克隆，而不是引用
            let group_upstreams = config_write.upstream_groups[index].upstreams.clone();

            // 释放config_write锁
            drop(config_write);

            // 遍历所有forward_states，更新负载均衡器
            for forward_state in app_state.forward_states.values() {
                if let Err(e) = forward_state
                    .upstream_manager
                    .update_group_load_balancer(&group_name, &group_upstreams)
                    .await
                {
                    warn!(
                        "Failed to update runtime load balancer for group '{}': {}",
                        group_name, e
                    );
                }
            }

            info!("API: Updated upstream group '{}'", name);

            // 记录响应体
            log_response_body(&detail);

            // 直接返回detail的所有权，避免克隆
            success_response(detail)
        }
        None => {
            let error = ErrorResponse::error(
                StatusCode::NOT_FOUND,
                error_types::NOT_FOUND,
                format!("Upstream group '{}' does not exist", name),
            );
            log_response_body(&error);
            not_found_error("Upstream group", &name)
        }
    }
}
