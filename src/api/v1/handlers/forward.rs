use crate::{
    api::v1::handlers::utils::{log_response_body, not_found_error, success_response_ref},
    api::v1::models::{ErrorResponse, SuccessResponse},
    api::v1::routes::AppState,
    config::ForwardConfig,
};
use axum::{
    extract::{Path, State},
    response::Response,
    Json,
};
use tracing::info;

/// 获取所有转发服务列表
///
/// Get all forwarding services list
#[utoipa::path(
    get,
    path = "/api/v1/forwards",
    tag = "Forwards",
    responses(
        (status = 200, description = "成功获取所有转发服务 | Successfully retrieved all forwarding services", body = SuccessResponse<Vec<ForwardConfig>>),
        (status = 500, description = "服务器内部错误 | Internal server error", body = ErrorResponse),
    )
)]
pub async fn list_forwards(
    State(app_state): State<AppState>,
) -> Json<SuccessResponse<Vec<ForwardConfig>>> {
    let forwards = app_state
        .config
        .read()
        .await
        .http_server
        .as_ref()
        .map(|s| s.forwards.clone())
        .unwrap_or_default();
    info!("API: Retrieved {} forward services", forwards.len());

    // 构建响应
    let response = SuccessResponse::success_with_data(forwards);

    // 记录响应体
    log_response_body(&response);

    Json(response)
}

/// 获取单个转发服务详情
///
/// Get a single forwarding service detail
#[utoipa::path(
    get,
    path = "/api/v1/forwards/{name}",
    tag = "Forwards",
    params(
        ("name" = String, Path, description = "转发服务名称 | Forwarding service name")
    ),
    responses(
        (status = 200, description = "成功获取转发服务 | Successfully retrieved forwarding service", body = SuccessResponse<ForwardConfig>),
        (status = 404, description = "转发服务不存在 | Forwarding service not found", body = ErrorResponse),
        (status = 500, description = "服务器内部错误 | Internal server error", body = ErrorResponse),
    )
)]
#[axum::debug_handler]
pub async fn get_forward(State(app_state): State<AppState>, Path(name): Path<String>) -> Response {
    let config_read = app_state.config.read().await;
    let forward = config_read
        .http_server
        .as_ref()
        .and_then(|s| s.forwards.iter().find(|f| f.name == name));

    match forward {
        Some(forward) => {
            info!("API: Retrieved forwarding service '{}'", name);

            // 记录响应体并直接使用引用
            let response = SuccessResponse::success_with_data(forward);
            log_response_body(&response);

            // 使用新的success_response_ref函数处理引用
            success_response_ref(forward)
        }
        None => {
            let error = ErrorResponse::error(
                axum::http::StatusCode::NOT_FOUND,
                "not_found",
                format!("Forwarding service '{}' does not exist", name),
            );
            log_response_body(&error);
            not_found_error("Forwarding service", &name)
        }
    }
}
