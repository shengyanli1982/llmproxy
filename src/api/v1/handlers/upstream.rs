use crate::api::v1::handlers::{
    util::next_id,
    validation::{check_config_integrity, check_upstream_references, validate_upstream_payload},
};
use crate::api::v1::models::{ApiError, ApiResponse};
use crate::config::{Config, UpstreamConfig};
use axum::http::StatusCode;
use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tracing::{error, info, warn};

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
    let request_id = next_id();
    info!(
        "[request: {}] API request started: Get all upstreams",
        request_id
    );
    let start_time = Instant::now();

    // 获取配置的只读锁
    let config_guard = config.read().unwrap();

    // 避免克隆整个配置对象，只克隆上游列表
    let upstreams_count = config_guard.upstreams.len();
    let upstreams = config_guard.upstreams.clone();

    // 立即释放锁
    drop(config_guard);

    let elapsed = start_time.elapsed();
    info!(
        "[request: {}] API request completed: Get all upstreams, time: {:?}, result count: {}",
        request_id, elapsed, upstreams_count
    );

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
    let request_id = next_id();
    info!(
        "[request: {}] API request started: Get upstream '{}'",
        request_id, name
    );
    let start_time = Instant::now();

    // 获取配置的只读锁
    let config_guard = config.read().unwrap();

    // 查找指定名称的上游
    let upstream = config_guard
        .upstreams
        .iter()
        .find(|u| u.name == name)
        .cloned();

    // 立即释放锁
    drop(config_guard);

    let elapsed = start_time.elapsed();
    match &upstream {
        Some(_) => info!(
            "[request: {}] API request completed: Get upstream '{}', time: {:?}, status: success",
            request_id, name, elapsed
        ),
        None => warn!(
            "[request: {}] API request completed: Get upstream '{}', time: {:?}, status: not found",
            request_id, name, elapsed
        ),
    }

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
    let request_id = next_id();
    info!(
        "[request: {}] API request started: Create upstream '{}'",
        request_id, upstream.name
    );
    let start_time = Instant::now();

    // 第一阶段：载荷验证
    if let Err(e) = validate_upstream_payload(&upstream) {
        let elapsed = start_time.elapsed();
        warn!(
            "[request: {}] API request failed: Create upstream '{}', time: {:?}, status: validation error, error: {}",
            request_id, upstream.name, elapsed, e
        );
        return Err(e);
    }

    // 获取配置的写锁
    let mut config_guard = config.write().unwrap();

    // 检查名称是否已存在
    if config_guard
        .upstreams
        .iter()
        .any(|u| u.name == upstream.name)
    {
        let elapsed = start_time.elapsed();
        warn!(
            "[request: {}] API request failed: Create upstream '{}', time: {:?}, status: resource conflict",
            request_id, upstream.name, elapsed
        );
        drop(config_guard); // 释放锁
        return Err(ApiError::resource_conflict(format!(
            "Upstream '{}' already exists",
            upstream.name
        )));
    }

    // 创建新配置的克隆，使用 Arc::make_mut 进行写时复制优化
    let new_config = Arc::make_mut(&mut *config_guard);

    // 添加新上游
    new_config.upstreams.push(upstream.clone());

    // 第二阶段：集成验证
    if let Err(e) = check_config_integrity(&new_config) {
        let elapsed = start_time.elapsed();
        warn!(
            "[request: {}] API request failed: Create upstream '{}', time: {:?}, status: integration validation error, error: {}",
            request_id, upstream.name, elapsed, e
        );
        drop(config_guard); // 释放锁
        return Err(e);
    }

    // 锁会在作用域结束时自动释放
    drop(config_guard);

    let elapsed = start_time.elapsed();
    info!(
        "[request: {}] API request completed: Create upstream '{}', time: {:?}, status: success",
        request_id, upstream.name, elapsed
    );

    // 返回创建的上游
    Ok(ApiResponse::with_code_and_message(
        StatusCode::CREATED.as_u16(),
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
    let request_id = next_id();
    info!(
        "[request: {}] API request started: Update upstream '{}'",
        request_id, name
    );
    let start_time = Instant::now();

    // 检查路径参数和请求体中的名称是否匹配
    if name != upstream.name {
        let elapsed = start_time.elapsed();
        warn!(
            "[request: {}] API request failed: Update upstream, time: {:?}, status: validation error, path parameter '{}' does not match request body name '{}'",
            request_id, elapsed, name, upstream.name
        );
        return Err(ApiError::validation_error(format!(
            "Path parameter name '{}' does not match request body name '{}'. The name field cannot be updated. To change the name, please delete the existing upstream and create a new one.",
            name, upstream.name
        )));
    }

    // 第一阶段：载荷验证
    if let Err(e) = validate_upstream_payload(&upstream) {
        let elapsed = start_time.elapsed();
        warn!(
            "[request: {}] API request failed: Update upstream '{}', time: {:?}, status: validation error, error: {}",
            request_id, name, elapsed, e
        );
        return Err(e);
    }

    // 获取配置的写锁
    let mut config_guard = config.write().unwrap();

    // 检查上游是否存在
    let upstream_index = match config_guard.upstreams.iter().position(|u| u.name == name) {
        Some(index) => index,
        None => {
            let elapsed = start_time.elapsed();
            warn!(
                "[request: {}] API request failed: Update upstream '{}', time: {:?}, status: resource not found",
                request_id, name, elapsed
            );
            drop(config_guard); // 释放锁
            return Err(ApiError::resource_not_found("Upstream", name));
        }
    };

    // 创建新配置的克隆，使用 Arc::make_mut 进行写时复制优化
    let new_config = Arc::make_mut(&mut *config_guard);

    // 更新上游
    new_config.upstreams[upstream_index] = upstream.clone();

    // 第二阶段：集成验证
    if let Err(e) = check_config_integrity(&new_config) {
        let elapsed = start_time.elapsed();
        warn!(
            "[request: {}] API request failed: Update upstream '{}', time: {:?}, status: integration validation error, error: {}",
            request_id, name, elapsed, e
        );
        drop(config_guard); // 释放锁
        return Err(e);
    }

    // 锁会在作用域结束时自动释放
    drop(config_guard);

    let elapsed = start_time.elapsed();
    info!(
        "[request: {}] API request completed: Update upstream '{}', time: {:?}, status: success",
        request_id, name, elapsed
    );

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
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 404, description = "Resource not found", body = ApiError),
        (status = 409, description = "Resource in use", body = ApiError)
    )
)]
pub async fn delete_upstream(
    State(config): State<Arc<RwLock<Arc<Config>>>>,
    Path(name): Path<String>,
) -> Result<ApiResponse<()>, ApiError> {
    let request_id = next_id();
    info!(
        "[request: {}] API request started: Delete upstream '{}'",
        request_id, name
    );
    let start_time = Instant::now();

    // 获取配置的写锁
    let mut config_guard = config.write().unwrap();

    // 检查上游是否存在
    let upstream_index = match config_guard.upstreams.iter().position(|u| u.name == name) {
        Some(index) => index,
        None => {
            let elapsed = start_time.elapsed();
            warn!(
                "[request: {}] API request failed: Delete upstream '{}', time: {:?}, status: resource not found",
                request_id, name, elapsed
            );
            drop(config_guard); // 释放锁
            return Err(ApiError::resource_not_found("Upstream", name));
        }
    };

    // 创建新配置的克隆，使用 Arc::make_mut 进行写时复制优化
    let new_config = Arc::make_mut(&mut *config_guard);

    // 检查上游引用
    if let Err(e) = check_upstream_references(&new_config, &name) {
        let elapsed = start_time.elapsed();
        warn!(
            "[request: {}] API request failed: Delete upstream '{}', time: {:?}, status: dependency check failed, error: {}",
            request_id, name, elapsed, e
        );
        drop(config_guard); // 释放锁
        return Err(e);
    }

    // 删除上游
    new_config.upstreams.remove(upstream_index);

    // 第二阶段：集成验证
    if let Err(e) = check_config_integrity(&new_config) {
        let elapsed = start_time.elapsed();
        error!(
            "[request: {}] API request failed: Delete upstream '{}', time: {:?}, status: integration validation error, error: {}",
            request_id, name, elapsed, e
        );
        drop(config_guard); // 释放锁
        return Err(e);
    }

    // 锁会在作用域结束时自动释放
    drop(config_guard);

    let elapsed = start_time.elapsed();
    info!(
        "[request: {}] API request completed: Delete upstream '{}', time: {:?}, status: success",
        request_id, name, elapsed
    );

    // 返回成功响应
    Ok(ApiResponse::with_message(
        None,
        format!("Upstream '{}' deleted successfully", name),
    ))
}
