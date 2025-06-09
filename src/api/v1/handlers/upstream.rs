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
#[utoipa::path(
    get,
    path = "/upstreams",
    tag = "upstreams",
    responses(
        (status = 200, description = "成功获取所有上游", body = ApiResponse<Vec<UpstreamConfig>>),
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
#[utoipa::path(
    get,
    path = "/upstreams/{name}",
    tag = "upstreams",
    params(
        ("name" = String, Path, description = "上游名称")
    ),
    responses(
        (status = 200, description = "成功获取上游", body = ApiResponse<UpstreamConfig>),
        (status = 404, description = "资源未找到", body = ApiError)
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
        None => Err(ApiError::resource_not_found("上游", name)),
    }
}

/// 创建上游
#[utoipa::path(
    post,
    path = "/upstreams",
    tag = "upstreams",
    request_body = UpstreamConfig,
    responses(
        (status = 201, description = "成功创建上游", body = ApiResponse<UpstreamConfig>),
        (status = 400, description = "请求无效", body = ApiError),
        (status = 409, description = "资源冲突", body = ApiError)
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
            "上游 '{}' 已存在",
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

    info!("上游 '{}' 已创建", upstream.name);

    // 返回创建的上游
    Ok(ApiResponse::with_code_and_message(
        201,
        Some(upstream.clone()),
        format!("上游 '{}' 创建成功", upstream.name),
    ))
}

/// 更新上游
#[utoipa::path(
    put,
    path = "/upstreams/{name}",
    tag = "upstreams",
    params(
        ("name" = String, Path, description = "要更新的上游名称")
    ),
    request_body = UpstreamConfig,
    responses(
        (status = 200, description = "成功更新上游", body = ApiResponse<UpstreamConfig>),
        (status = 400, description = "请求无效", body = ApiError),
        (status = 404, description = "资源未找到", body = ApiError)
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
            "路径参数名称 '{}' 与请求体中的名称 '{}' 不匹配",
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
        return Err(ApiError::resource_not_found("上游", name));
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

    info!("上游 '{}' 已更新", name);

    // 返回更新后的上游
    Ok(ApiResponse::with_message(
        Some(upstream),
        format!("上游 '{}' 更新成功", name),
    ))
}

/// 删除上游
#[utoipa::path(
    delete,
    path = "/upstreams/{name}",
    tag = "upstreams",
    params(
        ("name" = String, Path, description = "要删除的上游名称")
    ),
    responses(
        (status = 200, description = "成功删除上游", body = serde_json::Value, example = json!({
            "code": 200,
            "status": "success",
            "message": "上游 '...' 已成功删除",
            "data": null
        })),
        (status = 404, description = "资源未找到", body = ApiError),
        (status = 409, description = "资源被占用", body = ApiError)
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
        return Err(ApiError::resource_not_found("上游", name));
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

    info!("上游 '{}' 已删除", name);

    // 返回成功响应
    Ok(ApiResponse::with_message(
        None,
        format!("上游 '{}' 删除成功", name),
    ))
}
