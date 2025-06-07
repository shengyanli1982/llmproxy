use crate::apis::v1::{
    error::{ApiError, ApiResponse},
    types::{AdminTask, ConfigState, NameParam, PaginatedResponse, PaginationQuery, TaskSender},
};
use crate::config::UpstreamConfig;
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use tracing::debug;

const UPSTREAM_PATH: &str = "/api/v1/admin/upstreams";
const UPSTREAM_PATH_WITH_NAME: &str = "/api/v1/admin/upstreams/{name}";

// 上游路由
pub fn upstream_routes(config: ConfigState, sender: TaskSender) -> Router {
    Router::new()
        .route(UPSTREAM_PATH, get(list_upstreams))
        .route(UPSTREAM_PATH, post(create_upstream))
        .route(UPSTREAM_PATH_WITH_NAME, get(get_upstream))
        .route(UPSTREAM_PATH_WITH_NAME, put(update_upstream))
        .route(UPSTREAM_PATH_WITH_NAME, delete(delete_upstream))
        .with_state((config, sender))
}

// 列出所有上游
#[axum::debug_handler]
async fn list_upstreams(
    State((config, _)): State<(ConfigState, TaskSender)>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<UpstreamConfig>>>, ApiError> {
    // 获取配置并立即释放锁
    let upstreams = {
        let config_guard = config.read().await;

        config_guard.upstreams.clone()
    };

    let total = upstreams.len();
    let start = (query.page - 1) * query.page_size;
    let end = std::cmp::min(start + query.page_size, total);

    let items = if start < total {
        upstreams[start..end].to_vec()
    } else {
        Vec::new()
    };

    let response = PaginatedResponse::new(items, &query, total);
    Ok(Json(ApiResponse::ok(
        "Upstreams fetched successfully",
        Some(response),
    )))
}

// 创建上游
#[axum::debug_handler]
async fn create_upstream(
    State((_, sender)): State<(ConfigState, TaskSender)>,
    Json(upstream): Json<UpstreamConfig>,
) -> Result<Json<ApiResponse<UpstreamConfig>>, ApiError> {
    debug!("Received create upstream request: {}", upstream.name);

    // 发送创建任务
    sender
        .send(AdminTask::CreateUpstream(upstream.clone()))
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to send task: {}", e)))?;

    Ok(Json(ApiResponse::accepted(
        format!(
            "Upstream '{}' creation request accepted, processing",
            upstream.name
        ),
        Some(upstream),
    )))
}

// 获取单个上游
#[axum::debug_handler]
async fn get_upstream(
    State((config, _)): State<(ConfigState, TaskSender)>,
    Path(params): Path<NameParam>,
) -> Result<Json<ApiResponse<UpstreamConfig>>, ApiError> {
    // 获取配置并立即释放锁
    let upstream = {
        let config_guard = config.read().await;

        config_guard
            .upstreams
            .iter()
            .find(|u| u.name == params.name)
            .ok_or_else(|| ApiError::NotFound(format!("Upstream '{}' not found", params.name)))?
            .clone()
    };

    Ok(Json(ApiResponse::ok(
        format!("Upstream '{}' fetched successfully", params.name),
        Some(upstream),
    )))
}

// 更新上游
#[axum::debug_handler]
async fn update_upstream(
    State((_, sender)): State<(ConfigState, TaskSender)>,
    Path(params): Path<NameParam>,
    Json(upstream): Json<UpstreamConfig>,
) -> Result<Json<ApiResponse<UpstreamConfig>>, ApiError> {
    debug!("Received update upstream request: {}", params.name);

    // 检查名称一致性
    if upstream.name != params.name {
        return Err(ApiError::ValidationError(format!(
            "Name in URL ('{}') and request body ('{}') must match",
            params.name, upstream.name
        )));
    }

    // 发送更新任务
    sender
        .send(AdminTask::UpdateUpstream(
            params.name.clone(),
            upstream.clone(),
        ))
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to send task: {}", e)))?;

    Ok(Json(ApiResponse::accepted(
        format!(
            "Upstream '{}' update request accepted, processing",
            params.name
        ),
        Some(upstream),
    )))
}

// 删除上游
#[axum::debug_handler]
async fn delete_upstream(
    State((_, sender)): State<(ConfigState, TaskSender)>,
    Path(params): Path<NameParam>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    debug!("Received delete upstream request: {}", params.name);

    // 发送删除任务
    sender
        .send(AdminTask::DeleteUpstream(params.name.clone()))
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to send task: {}", e)))?;

    Ok(Json(ApiResponse::accepted(
        format!(
            "Upstream '{}' delete request accepted, processing",
            params.name
        ),
        None,
    )))
}
