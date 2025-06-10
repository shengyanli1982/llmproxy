use crate::{
    api::v1::models::{ApiResponse, ErrorResponse, UpstreamGroupDetail},
    config::{Config, UpstreamConfig},
    r#const::api,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::{collections::HashMap, sync::Arc};
use tracing::{info, warn};

/// 获取所有上游组列表
///
/// Get all upstream groups list
#[utoipa::path(
    get,
    path = "/api/v1/upstream-groups",
    tag = "UpstreamGroup",
    responses(
        (status = 200, description = "成功获取所有上游组 | Successfully retrieved all upstream groups", body = ApiResponse<Vec<UpstreamGroupDetail>>),
        (status = 500, description = "服务器内部错误 | Internal server error", body = ErrorResponse),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_upstream_groups(
    State(config): State<Arc<Config>>,
) -> Json<ApiResponse<Vec<UpstreamGroupDetail>>> {
    // 创建上游服务名称到配置的映射
    let upstream_map: HashMap<String, UpstreamConfig> = config
        .upstreams
        .iter()
        .map(|upstream| (upstream.name.clone(), upstream.clone()))
        .collect();

    // 将每个上游组转换为详情模型
    let groups: Vec<UpstreamGroupDetail> = config
        .upstream_groups
        .iter()
        .map(|group| UpstreamGroupDetail::from_config(group, &upstream_map))
        .collect();

    info!("API: Retrieved {} upstream groups", groups.len());
    Json(ApiResponse::success_with_data(
        groups,
        "Successfully retrieved upstream groups list",
    ))
}

/// 获取单个上游组详情
///
/// Get a single upstream group detail
#[utoipa::path(
    get,
    path = "/api/v1/upstream-groups/{name}",
    tag = "UpstreamGroup",
    params(
        ("name" = String, Path, description = "上游组名称 | Upstream group name")
    ),
    responses(
        (status = 200, description = "成功获取上游组 | Successfully retrieved upstream group", body = ApiResponse<UpstreamGroupDetail>),
        (status = 404, description = "上游组不存在 | Upstream group not found", body = ErrorResponse),
        (status = 500, description = "服务器内部错误 | Internal server error", body = ErrorResponse),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[axum::debug_handler]
pub async fn get_upstream_group(
    State(config): State<Arc<Config>>,
    Path(name): Path<String>,
) -> Response {
    // 查找指定名称的上游组
    match config
        .upstream_groups
        .iter()
        .find(|group| group.name == name)
    {
        Some(group) => {
            // 创建上游服务名称到配置的映射
            let upstream_map: HashMap<String, UpstreamConfig> = config
                .upstreams
                .iter()
                .map(|upstream| (upstream.name.clone(), upstream.clone()))
                .collect();

            // 转换为详情模型
            let detail = UpstreamGroupDetail::from_config(group, &upstream_map);
            info!("API: Retrieved upstream group '{}'", name);
            Json(ApiResponse::success_with_data(
                detail,
                "Successfully retrieved upstream group",
            ))
            .into_response()
        }
        None => {
            warn!("API: Upstream group '{}' not found", name);
            Json(ApiResponse::<()>::error(
                StatusCode::NOT_FOUND,
                api::error_types::NOT_FOUND,
                format!("Upstream group '{}' does not exist", name),
            ))
            .into_response()
        }
    }
}
