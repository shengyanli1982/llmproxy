use crate::apis::v1::{
    error::{ApiError, ApiResponse},
    types::{
        AdminTask, ConfigState, NameParam, PaginatedResponse, PaginationQuery, ServerManagerSender,
        ServerManagerTask, TaskSender,
    },
};
use crate::config::ForwardConfig;
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use tracing::debug;

const FORWARD_PATH: &str = "/api/v1/admin/forwards";
const FORWARD_PATH_WITH_NAME: &str = "/api/v1/admin/forwards/{name}";

// 转发规则路由
pub fn forward_routes(
    config: ConfigState,
    sender: TaskSender,
    server_manager_sender: ServerManagerSender,
) -> Router {
    Router::new()
        .route(FORWARD_PATH, get(list_forwards))
        .route(FORWARD_PATH, post(create_forward))
        .route(FORWARD_PATH_WITH_NAME, get(get_forward))
        .route(FORWARD_PATH_WITH_NAME, put(update_forward))
        .route(FORWARD_PATH_WITH_NAME, delete(delete_forward))
        .with_state((config, sender, server_manager_sender))
}

// 列出所有转发规则
#[axum::debug_handler]
async fn list_forwards(
    State((config, _, _)): State<(ConfigState, TaskSender, ServerManagerSender)>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<ForwardConfig>>>, ApiError> {
    // 获取配置并立即释放锁
    let forwards = {
        let config_guard = config.read().await;
        config_guard.http_server.forwards.clone()
    };

    let total = forwards.len();
    let start = (query.page - 1) * query.page_size;
    let end = std::cmp::min(start + query.page_size, total);

    let items = if start < total {
        forwards[start..end].to_vec()
    } else {
        Vec::new()
    };

    let response = PaginatedResponse::new(items, &query, total);
    Ok(Json(ApiResponse::ok(
        "Forwards fetched successfully",
        Some(response),
    )))
}

// 创建转发规则
#[axum::debug_handler]
async fn create_forward(
    State((_, sender, server_manager_sender)): State<(
        ConfigState,
        TaskSender,
        ServerManagerSender,
    )>,
    Json(forward): Json<ForwardConfig>,
) -> Result<Json<ApiResponse<ForwardConfig>>, ApiError> {
    debug!("Received create forward request: {}", forward.name);

    // 发送创建任务
    sender
        .send(AdminTask::CreateForward(forward.clone()))
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to send task: {}", e)))?;

    // 发送服务器启动任务
    server_manager_sender
        .send(ServerManagerTask::StartServer(forward.clone()))
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to send server management task: {}", e))
        })?;

    Ok(Json(ApiResponse::accepted(
        format!(
            "Forward '{}' creation request accepted, processing",
            forward.name
        ),
        Some(forward),
    )))
}

// 获取单个转发规则
#[axum::debug_handler]
async fn get_forward(
    State((config, _, _)): State<(ConfigState, TaskSender, ServerManagerSender)>,
    Path(params): Path<NameParam>,
) -> Result<Json<ApiResponse<ForwardConfig>>, ApiError> {
    // 获取配置并立即释放锁
    let forward = {
        let config_guard = config.read().await;

        config_guard
            .http_server
            .forwards
            .iter()
            .find(|f| f.name == params.name)
            .ok_or_else(|| ApiError::NotFound(format!("Forward '{}' not found", params.name)))?
            .clone()
    };

    Ok(Json(ApiResponse::ok(
        format!("Forward '{}' fetched successfully", params.name),
        Some(forward),
    )))
}

// 更新转发规则
#[axum::debug_handler]
async fn update_forward(
    State((config, sender, server_manager_sender)): State<(
        ConfigState,
        TaskSender,
        ServerManagerSender,
    )>,
    Path(params): Path<NameParam>,
    Json(forward): Json<ForwardConfig>,
) -> Result<Json<ApiResponse<ForwardConfig>>, ApiError> {
    debug!("Received update forward request: {}", params.name);

    // 检查名称一致性
    if forward.name != params.name {
        return Err(ApiError::ValidationError(format!(
            "Name in URL ('{}') and request body ('{}') must match",
            params.name, forward.name
        )));
    }

    // 获取当前配置并立即释放锁
    let current_forward = {
        let config_guard = config.read().await;

        // 查找当前的转发规则
        config_guard
            .http_server
            .forwards
            .iter()
            .find(|f| f.name == params.name)
            .ok_or_else(|| ApiError::NotFound(format!("Forward '{}' not found", params.name)))?
            .clone()
    };

    // 检查是否需要重启服务
    let need_restart =
        current_forward.address != forward.address || current_forward.port != forward.port;

    // 发送更新任务
    sender
        .send(AdminTask::UpdateForward(
            params.name.clone(),
            forward.clone(),
        ))
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to send task: {}", e)))?;

    // 如果地址或端口变化，需要停止旧服务并启动新服务
    if need_restart {
        // 发送停止旧服务任务
        server_manager_sender
            .send(ServerManagerTask::StopServer(params.name.clone()))
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to send stop server task: {}", e))
            })?;

        // 发送启动新服务任务
        server_manager_sender
            .send(ServerManagerTask::StartServer(forward.clone()))
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to send start server task: {}", e))
            })?;
    }

    Ok(Json(ApiResponse::accepted(
        format!(
            "Forward '{}' update request accepted, processing",
            params.name
        ),
        Some(forward),
    )))
}

// 删除转发规则
#[axum::debug_handler]
async fn delete_forward(
    State((_, sender, server_manager_sender)): State<(
        ConfigState,
        TaskSender,
        ServerManagerSender,
    )>,
    Path(params): Path<NameParam>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    debug!("Received delete forward request: {}", params.name);

    // 发送删除任务
    sender
        .send(AdminTask::DeleteForward(params.name.clone()))
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to send task: {}", e)))?;

    // 发送停止服务任务
    server_manager_sender
        .send(ServerManagerTask::StopServer(params.name.clone()))
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to send stop server task: {}", e)))?;

    Ok(Json(ApiResponse::accepted(
        format!(
            "Forward '{}' delete request accepted, processing",
            params.name
        ),
        None,
    )))
}
