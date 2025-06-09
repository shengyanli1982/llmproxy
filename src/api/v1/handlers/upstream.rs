use crate::api::v1::handlers::validation::{
    check_config_integrity, check_upstream_references, validate_upstream_payload,
};
use crate::api::v1::models::{ApiError, ApiResponse};
use crate::config::{Config, UpstreamConfig};
use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::{Arc, RwLock};
use tracing::info;

/// 获取所有上游
///
/// Get all upstreams
#[utoipa::path(
    get,
    path = "/upstreams",
    tag = "Upstreams",
    responses(
        (status = 200, description = "Successfully retrieved all upstreams", body = ApiResponse<Vec<UpstreamConfig>>),
    )
)]
pub async fn get_all_upstreams(
    State(config): State<Arc<RwLock<Arc<Config>>>>,
) -> Result<ApiResponse<Vec<UpstreamConfig>>, ApiError> {
    // 获取配置的只读锁
    let config_guard = config.read().unwrap();
    let config = Arc::clone(&config_guard);
    // 克隆上游列表以避免长时间持有锁
    let upstreams = config.upstreams.clone();

    // 返回上游列表
    Ok(ApiResponse::new(Some(upstreams)))
}

/// 获取单个上游
///
/// Get a single upstream
#[utoipa::path(
    get,
    path = "/upstreams/{name}",
    tag = "Upstreams",
    params(
        ("name" = String, Path, description = "Upstream name")
    ),
    responses(
        (status = 200, description = "Successfully retrieved upstream", body = ApiResponse<UpstreamConfig>),
        (status = 404, description = "Resource not found", body = ApiError)
    )
)]
pub async fn get_upstream(
    State(config): State<Arc<RwLock<Arc<Config>>>>,
    Path(name): Path<String>,
) -> Result<ApiResponse<UpstreamConfig>, ApiError> {
    // 获取配置的只读锁
    let config_guard = config.read().unwrap();
    let config = Arc::clone(&config_guard);

    // 查找指定名称的上游
    let upstream = config.upstreams.iter().find(|u| u.name == name).cloned();

    match upstream {
        Some(upstream) => Ok(ApiResponse::new(Some(upstream))),
        None => Err(ApiError::resource_not_found("Upstream", name)),
    }
}

/// 创建上游
///
/// Create an upstream
#[utoipa::path(
    post,
    path = "/upstreams",
    tag = "Upstreams",
    request_body = UpstreamConfig,
    responses(
        (status = 201, description = "Successfully created upstream", body = ApiResponse<UpstreamConfig>),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 409, description = "Resource conflict", body = ApiError)
    )
)]
pub async fn create_upstream(
    State(config): State<Arc<RwLock<Arc<Config>>>>,
    Json(upstream): Json<UpstreamConfig>,
) -> Result<ApiResponse<UpstreamConfig>, ApiError> {
    // 第一阶段：载荷验证
    validate_upstream_payload(&upstream)?;

    // 获取配置的写锁
    let mut config_guard = config.write().unwrap();
    let current_config = Arc::clone(&config_guard);

    // 检查名称是否已存在
    if current_config
        .upstreams
        .iter()
        .any(|u| u.name == upstream.name)
    {
        return Err(ApiError::resource_conflict(format!(
            "Upstream '{}' already exists",
            upstream.name
        )));
    }

    // 创建新配置的克隆
    let mut new_config = (*current_config).clone();

    // 添加新上游
    new_config.upstreams.push(upstream.clone());

    // 第二阶段：集成验证
    if let Err(e) = check_config_integrity(&new_config) {
        return Err(e);
    }

    // 更新配置
    *config_guard = Arc::new(new_config);

    info!("Upstream '{}' created", upstream.name);

    // 返回创建的上游
    Ok(ApiResponse::with_code_and_message(
        201,
        Some(upstream.clone()),
        format!("Upstream '{}' created successfully", upstream.name),
    ))
}

/// 更新上游
///
/// Update an upstream
#[utoipa::path(
    put,
    path = "/upstreams/{name}",
    tag = "Upstreams",
    params(
        ("name" = String, Path, description = "Name of the upstream to update")
    ),
    request_body = UpstreamConfig,
    responses(
        (status = 200, description = "Successfully updated upstream", body = ApiResponse<UpstreamConfig>),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 404, description = "Resource not found", body = ApiError)
    )
)]
pub async fn update_upstream(
    State(config): State<Arc<RwLock<Arc<Config>>>>,
    Path(name): Path<String>,
    Json(upstream): Json<UpstreamConfig>,
) -> Result<ApiResponse<UpstreamConfig>, ApiError> {
    // 检查路径参数和请求体中的名称是否匹配
    if name != upstream.name {
        return Err(ApiError::validation_error(format!(
            "Path parameter name '{}' does not match request body name '{}'. The name field cannot be updated. To change the name, please delete the existing upstream and create a new one.",
            name, upstream.name
        )));
    }

    // 第一阶段：载荷验证
    validate_upstream_payload(&upstream)?;

    // 获取配置的写锁
    let mut config_guard = config.write().unwrap();
    let current_config = Arc::clone(&config_guard);

    // 检查上游是否存在
    if !current_config.upstreams.iter().any(|u| u.name == name) {
        return Err(ApiError::resource_not_found("Upstream", name));
    }

    // 创建新配置的克隆
    let mut new_config = (*current_config).clone();

    // 查找并更新上游
    let index = new_config
        .upstreams
        .iter()
        .position(|u| u.name == name)
        .unwrap();
    new_config.upstreams[index] = upstream.clone();

    // 第二阶段：集成验证
    if let Err(e) = check_config_integrity(&new_config) {
        return Err(e);
    }

    // 更新配置
    *config_guard = Arc::new(new_config);

    info!("Upstream '{}' updated", name);

    // 返回更新后的上游
    Ok(ApiResponse::with_message(
        Some(upstream),
        format!("Upstream '{}' updated successfully", name),
    ))
}

/// 删除上游
///
/// Delete an upstream
#[utoipa::path(
    delete,
    path = "/upstreams/{name}",
    tag = "Upstreams",
    params(
        ("name" = String, Path, description = "Name of the upstream to delete")
    ),
    responses(
        (status = 200, description = "Successfully deleted upstream", body = serde_json::Value, example = json!({
            "code": 200,
            "status": "success",
            "message": "Upstream deleted successfully",
            "data": null
        })),
        (status = 404, description = "Resource not found", body = ApiError),
        (status = 409, description = "Resource in use", body = ApiError)
    )
)]
pub async fn delete_upstream(
    State(config): State<Arc<RwLock<Arc<Config>>>>,
    Path(name): Path<String>,
) -> Result<ApiResponse<()>, ApiError> {
    // 获取配置的写锁
    let mut config_guard = config.write().unwrap();
    let current_config = Arc::clone(&config_guard);

    // 检查上游是否存在
    if !current_config.upstreams.iter().any(|u| u.name == name) {
        return Err(ApiError::resource_not_found("Upstream", name));
    }

    // 检查上游是否被任何上游组引用
    check_upstream_references(&current_config, &name)?;

    // 创建新配置的克隆
    let mut new_config = (*current_config).clone();

    // 删除上游
    new_config.upstreams.retain(|u| u.name != name);

    // 第二阶段：集成验证
    if let Err(e) = check_config_integrity(&new_config) {
        return Err(e);
    }

    // 更新配置
    *config_guard = Arc::new(new_config);

    info!("Upstream '{}' deleted", name);

    // 返回成功响应
    Ok(ApiResponse::with_message(
        None,
        format!("Upstream '{}' deleted successfully", name),
    ))
}
