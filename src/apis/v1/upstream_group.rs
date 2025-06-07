use crate::apis::v1::{
    error::{ApiError, ApiResponse},
    types::{AdminTask, ConfigState, NameParam, PaginatedResponse, PaginationQuery, TaskSender},
};
use crate::config::UpstreamGroupConfig;
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use tracing::debug;

// 上游组路由
pub fn upstream_group_routes(config: ConfigState, sender: TaskSender) -> Router {
    Router::new()
        .route("/api/v1/admin/upstreamgroups", get(list_upstream_groups))
        .route("/api/v1/admin/upstreamgroups", post(create_upstream_group))
        .route(
            "/api/v1/admin/upstreamgroups/{name}",
            get(get_upstream_group),
        )
        .route(
            "/api/v1/admin/upstreamgroups/{name}",
            put(update_upstream_group),
        )
        .route(
            "/api/v1/admin/upstreamgroups/{name}",
            delete(delete_upstream_group),
        )
        .with_state((config, sender))
}

// 列出所有上游组
#[axum::debug_handler]
async fn list_upstream_groups(
    State((config, _)): State<(ConfigState, TaskSender)>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<UpstreamGroupConfig>>>, ApiError> {
    let groups = {
        let config_guard = config.read().await;

        config_guard.upstream_groups.clone()
    };

    let total = groups.len();
    let start = (query.page - 1) * query.page_size;
    let end = std::cmp::min(start + query.page_size, total);

    let items = if start < total {
        groups[start..end].to_vec()
    } else {
        Vec::new()
    };

    let response = PaginatedResponse::new(items, &query, total);
    Ok(Json(ApiResponse::ok(
        "Upstream groups fetched successfully",
        Some(response),
    )))
}

// 创建上游组
#[axum::debug_handler]
async fn create_upstream_group(
    State((_, sender)): State<(ConfigState, TaskSender)>,
    Json(group): Json<UpstreamGroupConfig>,
) -> Result<Json<ApiResponse<UpstreamGroupConfig>>, ApiError> {
    debug!("Received create upstream group request: {}", group.name);

    // 发送创建任务
    sender
        .send(AdminTask::CreateUpstreamGroup(group.clone()))
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to send task: {}", e)))?;

    Ok(Json(ApiResponse::accepted(
        format!(
            "Upstream group '{}' creation request accepted, processing",
            group.name
        ),
        Some(group),
    )))
}

// 获取单个上游组
#[axum::debug_handler]
async fn get_upstream_group(
    State((config, _)): State<(ConfigState, TaskSender)>,
    Path(params): Path<NameParam>,
) -> Result<Json<ApiResponse<UpstreamGroupConfig>>, ApiError> {
    // 获取配置并立即释放锁
    let group = {
        let config_guard = config.read().await;

        config_guard
            .upstream_groups
            .iter()
            .find(|g| g.name == params.name)
            .ok_or_else(|| {
                ApiError::NotFound(format!("Upstream group '{}' not found", params.name))
            })?
            .clone()
    };

    Ok(Json(ApiResponse::ok(
        format!("Upstream group '{}' fetched successfully", params.name),
        Some(group),
    )))
}

// 更新上游组
#[axum::debug_handler]
async fn update_upstream_group(
    State((_, sender)): State<(ConfigState, TaskSender)>,
    Path(params): Path<NameParam>,
    Json(group): Json<UpstreamGroupConfig>,
) -> Result<Json<ApiResponse<UpstreamGroupConfig>>, ApiError> {
    debug!("Updating upstream group: {}", params.name);

    // 检查名称一致性
    if group.name != params.name {
        return Err(ApiError::ValidationError(format!(
            "Name in URL ('{}') and request body ('{}') must match",
            params.name, group.name
        )));
    }

    // 发送更新任务
    sender
        .send(AdminTask::UpdateUpstreamGroup(
            params.name.clone(),
            group.clone(),
        ))
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to send task: {}", e)))?;

    Ok(Json(ApiResponse::accepted(
        format!(
            "Upstream group '{}' update request accepted and being processed",
            params.name
        ),
        Some(group),
    )))
}

// 删除上游组
#[axum::debug_handler]
async fn delete_upstream_group(
    State((_, sender)): State<(ConfigState, TaskSender)>,
    Path(params): Path<NameParam>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    debug!("Received delete upstream group request: {}", params.name);

    // 发送删除任务
    sender
        .send(AdminTask::DeleteUpstreamGroup(params.name.clone()))
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to send task: {}", e)))?;

    Ok(Json(ApiResponse::accepted(
        format!(
            "Upstream group '{}' delete request accepted, processing",
            params.name
        ),
        None,
    )))
}
