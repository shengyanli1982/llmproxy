use crate::{
    api::v1::models::{ApiResponse, ErrorResponse},
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
use tracing::{info, warn};

/// 获取所有上游服务列表
///
/// Get all upstream services list
#[utoipa::path(
    get,
    path = "/api/v1/upstreams",
    tag = "Upstream",
    responses(
        (status = 200, description = "成功获取所有上游服务 | Successfully retrieved all upstream services", body = ApiResponse<Vec<UpstreamConfig>>),
        (status = 500, description = "服务器内部错误 | Internal server error", body = ErrorResponse),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_upstreams(
    State(config): State<Arc<Config>>,
) -> Json<ApiResponse<Vec<UpstreamConfig>>> {
    let upstreams = config.upstreams.clone();
    info!("API: Retrieved {} upstream services", upstreams.len());
    Json(ApiResponse::success_with_data(
        upstreams,
        "Successfully retrieved upstream services list",
    ))
}

/// 获取单个上游服务详情
///
/// Get a single upstream service detail
#[utoipa::path(
    get,
    path = "/api/v1/upstreams/{name}",
    tag = "Upstream",
    params(
        ("name" = String, Path, description = "上游服务名称 | Upstream service name")
    ),
    responses(
        (status = 200, description = "成功获取上游服务 | Successfully retrieved upstream service", body = ApiResponse<UpstreamConfig>),
        (status = 404, description = "上游服务不存在 | Upstream service not found", body = ErrorResponse),
        (status = 500, description = "服务器内部错误 | Internal server error", body = ErrorResponse),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[axum::debug_handler]
pub async fn get_upstream(State(config): State<Arc<Config>>, Path(name): Path<String>) -> Response {
    // 查找指定名称的上游服务
    match config
        .upstreams
        .iter()
        .find(|upstream| upstream.name == name)
    {
        Some(upstream) => {
            info!("API: Retrieved upstream service '{}'", name);
            Json(ApiResponse::success_with_data(
                upstream.clone(),
                "Successfully retrieved upstream service",
            ))
            .into_response()
        }
        None => {
            warn!("API: Upstream service '{}' not found", name);
            Json(ApiResponse::<()>::error(
                StatusCode::NOT_FOUND,
                api::error_types::NOT_FOUND,
                format!("Upstream service '{}' does not exist", name),
            ))
            .into_response()
        }
    }
}
