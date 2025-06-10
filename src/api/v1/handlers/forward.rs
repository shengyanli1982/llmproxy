use crate::{
    api::v1::models::{ApiResponse, ErrorResponse},
    config::{Config, ForwardConfig},
    r#const::api,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use tracing::{info, warn};

/// 获取所有转发规则列表
///
/// Get all forwarding rules list
#[utoipa::path(
    get,
    path = "/api/v1/forwards",
    tag = "Forwards",
    responses(
        (status = 200, description = "成功获取所有转发规则 | Successfully retrieved all forwarding rules", body = ApiResponse<Vec<ForwardConfig>>),
        (status = 500, description = "服务器内部错误 | Internal server error", body = ErrorResponse),
    )
)]
pub async fn list_forwards(
    State(config): State<Arc<Config>>,
) -> Json<ApiResponse<Vec<ForwardConfig>>> {
    let forwards = config.http_server.forwards.clone();
    info!("API: Retrieved {} forwarding rules", forwards.len());
    Json(ApiResponse::success_with_data(
        forwards,
        "Successfully retrieved forwarding rules list",
    ))
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
        (status = 200, description = "成功获取转发规则 | Successfully retrieved forwarding rule", body = ApiResponse<ForwardConfig>),
        (status = 404, description = "转发规则不存在 | Forwarding rule not found", body = ErrorResponse),
        (status = 500, description = "服务器内部错误 | Internal server error", body = ErrorResponse),
    )
)]
#[axum::debug_handler]
pub async fn get_forward(State(config): State<Arc<Config>>, Path(name): Path<String>) -> Response {
    // 查找指定名称的转发规则
    match config
        .http_server
        .forwards
        .iter()
        .find(|forward| forward.name == name)
    {
        Some(forward) => {
            info!("API: Retrieved forwarding rule '{}'", name);
            Json(ApiResponse::success_with_data(
                forward.clone(),
                "Successfully retrieved forwarding rule",
            ))
            .into_response()
        }
        None => {
            warn!("API: Forwarding rule '{}' not found", name);
            Json(ApiResponse::<()>::error(
                StatusCode::NOT_FOUND,
                api::error_types::NOT_FOUND,
                format!("Forwarding rule '{}' does not exist", name),
            ))
            .into_response()
        }
    }
}
