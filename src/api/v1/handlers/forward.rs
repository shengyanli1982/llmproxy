use crate::{
    api::v1::handlers::utils::{log_response_body, not_found_error, success_response},
    api::v1::models::{ErrorResponse, SuccessResponse},
    config::{Config, ForwardConfig},
};
use axum::{
    extract::{Path, State},
    response::Response,
    Json,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// 获取所有转发规则列表
///
/// Get all forwarding rules list
#[utoipa::path(
    get,
    path = "/api/v1/forwards",
    tag = "Forwards",
    responses(
        (status = 200, description = "成功获取所有转发规则 | Successfully retrieved all forwarding rules", body = SuccessResponse<Vec<ForwardConfig>>),
        (status = 500, description = "服务器内部错误 | Internal server error", body = ErrorResponse),
    )
)]
pub async fn list_forwards(
    State(config): State<Arc<RwLock<Config>>>,
) -> Json<SuccessResponse<Vec<ForwardConfig>>> {
    let forwards = config
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

/// 获取单个转发规则详情
///
/// Get a single forwarding rule detail
#[utoipa::path(
    get,
    path = "/api/v1/forwards/{name}",
    tag = "Forwards",
    params(
        ("name" = String, Path, description = "转发规则名称 | Forwarding rule name")
    ),
    responses(
        (status = 200, description = "成功获取转发规则 | Successfully retrieved forwarding rule", body = SuccessResponse<ForwardConfig>),
        (status = 404, description = "转发规则不存在 | Forwarding rule not found", body = ErrorResponse),
        (status = 500, description = "服务器内部错误 | Internal server error", body = ErrorResponse),
    )
)]
#[axum::debug_handler]
pub async fn get_forward(
    State(config): State<Arc<RwLock<Config>>>,
    Path(name): Path<String>,
) -> Response {
    let config_read = config.read().await;
    let forward = config_read
        .http_server
        .as_ref()
        .and_then(|s| s.forwards.iter().find(|f| f.name == name));

    match forward {
        Some(forward) => {
            info!("API: Retrieved forwarding rule '{}'", name);

            // 记录响应体
            let response = SuccessResponse::success_with_data(forward.clone());
            log_response_body(&response);

            success_response(forward)
        }
        None => {
            let error = ErrorResponse::error(
                axum::http::StatusCode::NOT_FOUND,
                "not_found",
                format!("Forwarding rule '{}' does not exist", name),
            );
            log_response_body(&error);
            not_found_error("Forwarding rule", &name)
        }
    }
}
