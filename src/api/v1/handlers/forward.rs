use crate::api::v1::handlers::validation::{check_config_integrity, validate_forward_payload};
use crate::api::v1::models::{ApiError, ApiResponse};
use crate::config::{Config, ForwardConfig};
use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::{Arc, RwLock};
use tracing::info;

/// 获取所有转发服务
#[utoipa::path(
    get,
    path = "/forwards",
    tag = "forwards",
    responses(
        (status = 200, description = "成功获取所有转发服务", body = ApiResponse<Vec<ForwardConfig>>)
    )
)]
pub async fn get_all_forwards(
    State(config): State<Arc<RwLock<Arc<Config>>>>,
) -> Result<ApiResponse<Vec<ForwardConfig>>, ApiError> {
    // 获取配置的只读锁
    let config_guard = config.read().unwrap();
    let config = Arc::clone(&config_guard);
    // 克隆转发服务列表以避免长时间持有锁
    let forwards = config.http_server.forwards.clone();

    // 返回转发服务列表
    Ok(ApiResponse::new(Some(forwards)))
}

/// 获取单个转发服务
#[utoipa::path(
    get,
    path = "/forwards/{name}",
    tag = "forwards",
    params(
        ("name" = String, Path, description = "转发服务名称")
    ),
    responses(
        (status = 200, description = "成功获取转发服务", body = ApiResponse<ForwardConfig>),
        (status = 404, description = "资源未找到", body = ApiError)
    )
)]
pub async fn get_forward(
    State(config): State<Arc<RwLock<Arc<Config>>>>,
    Path(name): Path<String>,
) -> Result<ApiResponse<ForwardConfig>, ApiError> {
    // 获取配置的只读锁
    let config_guard = config.read().unwrap();
    let config = Arc::clone(&config_guard);

    // 查找指定名称的转发服务
    let forward = config
        .http_server
        .forwards
        .iter()
        .find(|f| f.name == name)
        .cloned();

    match forward {
        Some(forward) => Ok(ApiResponse::new(Some(forward))),
        None => Err(ApiError::resource_not_found("转发服务", name)),
    }
}

/// 创建转发服务
#[utoipa::path(
    post,
    path = "/forwards",
    tag = "forwards",
    request_body = ForwardConfig,
    responses(
        (status = 201, description = "成功创建转发服务", body = ApiResponse<ForwardConfig>),
        (status = 400, description = "请求无效", body = ApiError),
        (status = 409, description = "资源冲突", body = ApiError)
    )
)]
pub async fn create_forward(
    State(config): State<Arc<RwLock<Arc<Config>>>>,
    Json(forward): Json<ForwardConfig>,
) -> Result<ApiResponse<ForwardConfig>, ApiError> {
    // 第一阶段：载荷验证
    validate_forward_payload(&forward)?;

    // 获取配置的写锁
    let mut config_guard = config.write().unwrap();
    let current_config = Arc::clone(&config_guard);

    // 检查名称是否已存在
    if current_config
        .http_server
        .forwards
        .iter()
        .any(|f| f.name == forward.name)
    {
        return Err(ApiError::resource_conflict(format!(
            "转发服务 '{}' 已存在",
            forward.name
        )));
    }

    // 创建新配置的克隆
    let mut new_config = (*current_config).clone();

    // 添加新转发服务
    new_config.http_server.forwards.push(forward.clone());

    // 第二阶段：集成验证
    if let Err(e) = check_config_integrity(&new_config) {
        return Err(e);
    }

    // 更新配置
    *config_guard = Arc::new(new_config);

    info!("转发服务 '{}' 已创建", forward.name);

    // 返回创建的转发服务
    Ok(ApiResponse::with_code_and_message(
        201,
        Some(forward.clone()),
        format!("转发服务 '{}' 创建成功", forward.name),
    ))
}

/// 更新转发服务
#[utoipa::path(
    put,
    path = "/forwards/{name}",
    tag = "forwards",
    params(
        ("name" = String, Path, description = "要更新的转发服务名称")
    ),
    request_body = ForwardConfig,
    responses(
        (status = 200, description = "成功更新转发服务", body = ApiResponse<ForwardConfig>),
        (status = 400, description = "请求无效", body = ApiError),
        (status = 404, description = "资源未找到", body = ApiError)
    )
)]
pub async fn update_forward(
    State(config): State<Arc<RwLock<Arc<Config>>>>,
    Path(name): Path<String>,
    Json(forward): Json<ForwardConfig>,
) -> Result<ApiResponse<ForwardConfig>, ApiError> {
    // 检查路径参数和请求体中的名称是否匹配
    if name != forward.name {
        return Err(ApiError::validation_error(format!(
            "路径参数名称 '{}' 与请求体中的名称 '{}' 不匹配",
            name, forward.name
        )));
    }

    // 第一阶段：载荷验证
    validate_forward_payload(&forward)?;

    // 获取配置的写锁
    let mut config_guard = config.write().unwrap();
    let current_config = Arc::clone(&config_guard);

    // 检查转发服务是否存在
    if !current_config
        .http_server
        .forwards
        .iter()
        .any(|f| f.name == name)
    {
        return Err(ApiError::resource_not_found("转发服务", name));
    }

    // 创建新配置的克隆
    let mut new_config = (*current_config).clone();

    // 查找并更新转发服务
    let index = new_config
        .http_server
        .forwards
        .iter()
        .position(|f| f.name == name)
        .unwrap();
    new_config.http_server.forwards[index] = forward.clone();

    // 第二阶段：集成验证
    if let Err(e) = check_config_integrity(&new_config) {
        return Err(e);
    }

    // 更新配置
    *config_guard = Arc::new(new_config);

    info!("转发服务 '{}' 已更新", name);

    // 返回更新后的转发服务
    Ok(ApiResponse::with_message(
        Some(forward),
        format!("转发服务 '{}' 更新成功", name),
    ))
}

/// 删除转发服务
#[utoipa::path(
    delete,
    path = "/forwards/{name}",
    tag = "forwards",
    params(
        ("name" = String, Path, description = "要删除的转发服务名称")
    ),
    responses(
        (status = 200, description = "成功删除转发服务", body = serde_json::Value, example = json!({
            "code": 200,
            "status": "success",
            "message": "转发服务 '...' 已成功删除",
            "data": null
        })),
        (status = 404, description = "资源未找到", body = ApiError)
    )
)]
pub async fn delete_forward(
    State(config): State<Arc<RwLock<Arc<Config>>>>,
    Path(name): Path<String>,
) -> Result<ApiResponse<()>, ApiError> {
    // 获取配置的写锁
    let mut config_guard = config.write().unwrap();
    let current_config = Arc::clone(&config_guard);

    // 检查转发服务是否存在
    if !current_config
        .http_server
        .forwards
        .iter()
        .any(|f| f.name == name)
    {
        return Err(ApiError::resource_not_found("转发服务", name));
    }

    // 创建新配置的克隆
    let mut new_config = (*current_config).clone();

    // 删除转发服务
    new_config.http_server.forwards.retain(|f| f.name != name);

    // 第二阶段：集成验证
    if let Err(e) = check_config_integrity(&new_config) {
        return Err(e);
    }

    // 更新配置
    *config_guard = Arc::new(new_config);

    info!("转发服务 '{}' 已删除", name);

    // 返回成功响应
    Ok(ApiResponse::with_message(
        None,
        format!("转发服务 '{}' 删除成功", name),
    ))
}
