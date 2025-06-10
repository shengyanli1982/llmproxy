use crate::{
    api::v1::models::ApiResponse,
    config::{Config, UpstreamConfig},
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
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
        (status = 500, description = "服务器内部错误 | Internal server error", body = ApiResponse<()>),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_upstreams(State(config): State<Arc<Config>>) -> ApiResponse<Vec<UpstreamConfig>> {
    let upstreams = config.upstreams.clone();
    info!("API: Retrieved {} upstream services", upstreams.len());
    ApiResponse::success_with_data(upstreams, "Successfully retrieved upstream services list")
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
        (status = 404, description = "上游服务不存在 | Upstream service not found", body = ApiResponse<()>),
        (status = 500, description = "服务器内部错误 | Internal server error", body = ApiResponse<()>),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_upstream(
    State(config): State<Arc<Config>>,
    Path(name): Path<String>,
) -> ApiResponse<UpstreamConfig> {
    // 查找指定名称的上游服务
    match config
        .upstreams
        .iter()
        .find(|upstream| upstream.name == name)
    {
        Some(upstream) => {
            info!("API: Retrieved upstream service '{}'", name);
            ApiResponse::success_with_data(
                upstream.clone(),
                "Successfully retrieved upstream service",
            )
        }
        None => {
            warn!("API: Upstream service '{}' not found", name);
            ApiResponse::error(
                StatusCode::NOT_FOUND,
                "Not Found",
                format!("Upstream service '{}' does not exist", name),
            )
        }
    }
}
