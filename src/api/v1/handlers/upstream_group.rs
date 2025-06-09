use crate::api::v1::handlers::validation::{
    check_config_integrity, check_upstream_group_references, validate_upstream_group_payload,
};
use crate::api::v1::models::{ApiError, ApiResponse};
use crate::config::{Config, UpstreamGroupConfig};
use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::{Arc, RwLock};
use tracing::info;

/// 获取所有上游组
///
/// Get all upstream groups
#[utoipa::path(
    get,
    path = "/upstream-groups",
    tag = "UpstreamGroups",
    responses(
        (status = 200, description = "Successfully retrieved all upstream groups", body = ApiResponse<Vec<UpstreamGroupConfig>>)
    )
)]
pub async fn get_all_upstream_groups(
    State(config): State<Arc<RwLock<Arc<Config>>>>,
) -> Result<ApiResponse<Vec<UpstreamGroupConfig>>, ApiError> {
    // 获取配置的只读锁
    let config_guard = config.read().unwrap();
    let config = Arc::clone(&config_guard);
    // 克隆上游组列表以避免长时间持有锁
    let upstream_groups = config.upstream_groups.clone();

    // 返回上游组列表
    Ok(ApiResponse::new(Some(upstream_groups)))
}

/// 获取单个上游组
///
/// Get a single upstream group by name
#[utoipa::path(
    get,
    path = "/upstream-groups/{name}",
    tag = "UpstreamGroups",
    params(
        ("name" = String, Path, description = "Upstream group name")
    ),
    responses(
        (status = 200, description = "Successfully retrieved upstream group", body = ApiResponse<UpstreamGroupConfig>),
        (status = 404, description = "Resource not found", body = ApiError)
    )
)]
pub async fn get_upstream_group(
    State(config): State<Arc<RwLock<Arc<Config>>>>,
    Path(name): Path<String>,
) -> Result<ApiResponse<UpstreamGroupConfig>, ApiError> {
    // 获取配置的只读锁
    let config_guard = config.read().unwrap();
    let config = Arc::clone(&config_guard);

    // 查找指定名称的上游组
    let upstream_group = config
        .upstream_groups
        .iter()
        .find(|g| g.name == name)
        .cloned();

    match upstream_group {
        Some(group) => Ok(ApiResponse::new(Some(group))),
        None => Err(ApiError::resource_not_found("Upstream group", name)),
    }
}

/// 创建上游组
///
/// Create a new upstream group
#[utoipa::path(
    post,
    path = "/upstream-groups",
    tag = "UpstreamGroups",
    request_body = UpstreamGroupConfig,
    responses(
        (status = 201, description = "Successfully created upstream group", body = ApiResponse<UpstreamGroupConfig>),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 409, description = "Resource conflict", body = ApiError)
    )
)]
pub async fn create_upstream_group(
    State(config): State<Arc<RwLock<Arc<Config>>>>,
    Json(group): Json<UpstreamGroupConfig>,
) -> Result<ApiResponse<UpstreamGroupConfig>, ApiError> {
    // 第一阶段：载荷验证
    validate_upstream_group_payload(&group)?;

    // 获取配置的写锁
    let mut config_guard = config.write().unwrap();
    let current_config = Arc::clone(&config_guard);

    // 检查名称是否已存在
    if current_config
        .upstream_groups
        .iter()
        .any(|g| g.name == group.name)
    {
        return Err(ApiError::resource_conflict(format!(
            "Upstream group '{}' already exists",
            group.name
        )));
    }

    // 创建新配置的克隆
    let mut new_config = (*current_config).clone();

    // 添加新上游组
    new_config.upstream_groups.push(group.clone());

    // 第二阶段：集成验证
    if let Err(e) = check_config_integrity(&new_config) {
        return Err(e);
    }

    // 更新配置
    *config_guard = Arc::new(new_config);

    info!("Upstream group '{}' created", group.name);

    // 返回创建的上游组
    Ok(ApiResponse::with_code_and_message(
        201,
        Some(group.clone()),
        format!("Upstream group '{}' created successfully", group.name),
    ))
}

/// 更新上游组
///
/// Update an upstream group by name
#[utoipa::path(
    put,
    path = "/upstream-groups/{name}",
    tag = "UpstreamGroups",
    params(
        ("name" = String, Path, description = "Name of the upstream group to update")
    ),
    request_body = UpstreamGroupConfig,
    responses(
        (status = 200, description = "Successfully updated upstream group", body = ApiResponse<UpstreamGroupConfig>),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 404, description = "Resource not found", body = ApiError)
    )
)]
pub async fn update_upstream_group(
    State(config): State<Arc<RwLock<Arc<Config>>>>,
    Path(name): Path<String>,
    Json(group): Json<UpstreamGroupConfig>,
) -> Result<ApiResponse<UpstreamGroupConfig>, ApiError> {
    // 检查路径参数和请求体中的名称是否匹配
    if name != group.name {
        return Err(ApiError::validation_error(format!(
            "Path parameter name '{}' does not match request body name '{}'. The name field cannot be updated. To change the name, please delete the existing upstream group and create a new one.",
            name, group.name
        )));
    }

    // 第一阶段：载荷验证
    validate_upstream_group_payload(&group)?;

    // 获取配置的写锁
    let mut config_guard = config.write().unwrap();
    let current_config = Arc::clone(&config_guard);

    // 检查上游组是否存在
    if !current_config
        .upstream_groups
        .iter()
        .any(|g| g.name == name)
    {
        return Err(ApiError::resource_not_found("Upstream group", name));
    }

    // 创建新配置的克隆
    let mut new_config = (*current_config).clone();

    // 查找并更新上游组
    let index = new_config
        .upstream_groups
        .iter()
        .position(|g| g.name == name)
        .unwrap();
    new_config.upstream_groups[index] = group.clone();

    // 第二阶段：集成验证
    if let Err(e) = check_config_integrity(&new_config) {
        return Err(e);
    }

    // 更新配置
    *config_guard = Arc::new(new_config);

    info!("Upstream group '{}' updated", name);

    // 返回更新后的上游组
    Ok(ApiResponse::with_message(
        Some(group),
        format!("Upstream group '{}' updated successfully", name),
    ))
}

/// 删除上游组
///
/// Delete an upstream group by name
#[utoipa::path(
    delete,
    path = "/upstream-groups/{name}",
    tag = "UpstreamGroups",
    params(
        ("name" = String, Path, description = "Name of the upstream group to delete")
    ),
    responses(
        (status = 200, description = "Successfully deleted upstream group", body = serde_json::Value, example = json!({
            "code": 200,
            "status": "success",
            "message": "Upstream group deleted successfully",
            "data": null
        })),
        (status = 404, description = "Resource not found", body = ApiError),
        (status = 409, description = "Resource in use", body = ApiError)
    )
)]
pub async fn delete_upstream_group(
    State(config): State<Arc<RwLock<Arc<Config>>>>,
    Path(name): Path<String>,
) -> Result<ApiResponse<()>, ApiError> {
    // 获取配置的写锁
    let mut config_guard = config.write().unwrap();
    let current_config = Arc::clone(&config_guard);

    // 检查上游组是否存在
    if !current_config
        .upstream_groups
        .iter()
        .any(|g| g.name == name)
    {
        return Err(ApiError::resource_not_found("Upstream group", name));
    }

    // 检查上游组是否被任何转发服务引用
    check_upstream_group_references(&current_config, &name)?;

    // 创建新配置的克隆
    let mut new_config = (*current_config).clone();

    // 删除上游组
    new_config.upstream_groups.retain(|g| g.name != name);

    // 第二阶段：集成验证
    if let Err(e) = check_config_integrity(&new_config) {
        return Err(e);
    }

    // 更新配置
    *config_guard = Arc::new(new_config);

    info!("Upstream group '{}' deleted", name);

    // 返回成功响应
    Ok(ApiResponse::with_message(
        None,
        format!("Upstream group '{}' deleted successfully", name),
    ))
}
