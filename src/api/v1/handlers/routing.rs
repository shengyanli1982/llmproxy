use crate::{
    api::v1::{
        handlers::utils::{
            decode_base64_to_path, log_request_body, log_response_body, not_found_error,
            success_response,
        },
        models::{ErrorResponse, SuccessResponse, UpdateRoutePayload},
    },
    config::{http_server::RoutingRule, Config},
    r#const::api::error_types,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use validator::Validate;

// 解码 base64 路径
#[inline(always)]
fn decode_path(encoded_path: &str) -> Result<String, Response> {
    match decode_base64_to_path(encoded_path) {
        Ok(p) => Ok(p),
        Err(e) => {
            let error = ErrorResponse::error(
                StatusCode::BAD_REQUEST,
                error_types::BAD_REQUEST,
                format!("Invalid base64 path: {}", e),
            );
            log_response_body(&error);
            Err(Json(error).into_response())
        }
    }
}

// 获取 HTTP 服务器配置
#[inline(always)]
fn get_http_server(
    config_write: &mut Config,
) -> Result<&mut crate::config::http_server::HttpServerConfig, Response> {
    match config_write.http_server.as_mut() {
        Some(server) => Ok(server),
        None => {
            let error = ErrorResponse::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                error_types::INTERNAL_SERVER_ERROR,
                "HTTP server configuration is missing",
            );
            log_response_body(&error);
            Err(Json(error).into_response())
        }
    }
}

// 检查上游组是否存在
#[inline(always)]
fn check_upstream_group_exists(config_write: &Config, target_group: &str) -> Result<(), Response> {
    let upstream_group_exists = config_write
        .upstream_groups
        .iter()
        .any(|g| g.name == target_group);

    if !upstream_group_exists {
        let error = ErrorResponse::error(
            StatusCode::BAD_REQUEST,
            error_types::BAD_REQUEST,
            format!("Target upstream group '{}' does not exist", target_group),
        );
        log_response_body(&error);
        return Err(Json(error).into_response());
    }
    Ok(())
}

// 处理转发服务不存在的错误
#[inline(always)]
fn forward_not_found(forward_name: &str) -> Response {
    let error = ErrorResponse::error(
        StatusCode::NOT_FOUND,
        error_types::NOT_FOUND,
        format!("Forward '{}' does not exist", forward_name),
    );
    log_response_body(&error);
    not_found_error("Forward", forward_name)
}

// 处理路由规则不存在的错误
#[inline(always)]
fn route_not_found(path: &str, forward_name: &str) -> Response {
    let error = ErrorResponse::error(
        StatusCode::NOT_FOUND,
        error_types::NOT_FOUND,
        format!(
            "Route '{}' does not exist in forward '{}'",
            path, forward_name
        ),
    );
    log_response_body(&error);
    not_found_error("Route", path)
}

/// 获取指定转发服务的所有路由规则
///
/// Get all routing rules for a specified forwarding service
#[utoipa::path(
    get,
    path = "/api/v1/forwards/{name}/routes",
    tag = "Routes",
    params(
        ("name" = String, Path, description = "转发服务名称")
    ),
    responses(
        (status = 200, description = "成功获取所有路由规则", body = SuccessResponse<Vec<RoutingRule>>),
        (status = 404, description = "转发服务不存在", body = ErrorResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse),
    )
)]
pub async fn list_routes(
    State(config): State<Arc<RwLock<Config>>>,
    Path(forward_name): Path<String>,
) -> Response {
    // 获取配置的读锁
    let config_read = config.read().await;

    // 查找指定的转发服务
    let forward = config_read
        .http_server
        .as_ref()
        .and_then(|s| s.forwards.iter().find(|f| f.name == forward_name));

    match forward {
        Some(forward) => {
            // 提取路由规则，如果不存在则返回空数组
            let routes: Vec<RoutingRule> = forward
                .routing
                .as_ref()
                .map(|rules| rules.clone())
                .unwrap_or_default();

            info!(
                "API: Retrieved {} routing rules for forward '{}'",
                routes.len(),
                forward_name
            );

            // 构建响应
            let response = SuccessResponse::success_with_data(routes);

            // 记录响应体
            log_response_body(&response);

            Json(response).into_response()
        }
        None => forward_not_found(&forward_name),
    }
}

/// 获取指定转发服务中特定路由规则的详细信息
///
/// Get detailed information for a specific routing rule in a specified forwarding service
#[utoipa::path(
    get,
    path = "/api/v1/forwards/{name}/routes/{path}",
    tag = "Routes",
    params(
        ("name" = String, Path, description = "转发服务名称"),
        ("path" = String, Path, description = "Base64编码的路径模式")
    ),
    responses(
        (status = 200, description = "成功获取路由规则", body = SuccessResponse<RoutingRule>),
        (status = 400, description = "无效的Base64编码", body = ErrorResponse),
        (status = 404, description = "转发服务或路由规则不存在", body = ErrorResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse),
    )
)]
pub async fn get_route(
    State(config): State<Arc<RwLock<Config>>>,
    Path((forward_name, encoded_path)): Path<(String, String)>,
) -> Response {
    // 解码路径
    let path = match decode_path(&encoded_path) {
        Ok(p) => p,
        Err(response) => return response,
    };

    // 获取配置的读锁
    let config_read = config.read().await;

    // 查找指定的转发服务
    let forward = config_read
        .http_server
        .as_ref()
        .and_then(|s| s.forwards.iter().find(|f| f.name == forward_name));

    match forward {
        Some(forward) => {
            // 查找指定的路由规则
            let route = forward
                .routing
                .as_ref()
                .and_then(|rules| rules.iter().find(|r| r.path == path));

            match route {
                Some(route) => {
                    info!(
                        "API: Retrieved routing rule '{}' for forward '{}'",
                        path, forward_name
                    );

                    // 记录响应体
                    let response = SuccessResponse::success_with_data(route);
                    log_response_body(&response);

                    success_response(route)
                }
                None => route_not_found(&path, &forward_name),
            }
        }
        None => forward_not_found(&forward_name),
    }
}

/// 在指定转发服务中创建新的路由规则
///
/// Create a new routing rule in a specified forwarding service
#[utoipa::path(
    post,
    path = "/api/v1/forwards/{name}/routes",
    tag = "Routes",
    params(
        ("name" = String, Path, description = "转发服务名称")
    ),
    request_body = RoutingRule,
    responses(
        (status = 201, description = "成功创建路由规则", body = SuccessResponse<RoutingRule>),
        (status = 400, description = "无效的请求参数", body = ErrorResponse),
        (status = 404, description = "转发服务或目标上游组不存在", body = ErrorResponse),
        (status = 409, description = "路由规则已存在", body = ErrorResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse),
    )
)]
pub async fn create_route(
    State(config): State<Arc<RwLock<Config>>>,
    Path(forward_name): Path<String>,
    Json(payload): Json<RoutingRule>,
) -> Response {
    // 记录请求体
    log_request_body(&payload);

    // 验证请求体
    if let Err(e) = payload.validate() {
        let error = ErrorResponse::from_validation_errors(e);
        log_response_body(&error);
        return Json(error).into_response();
    }

    // 获取配置的写锁
    let mut config_write = config.write().await;

    // 先检查上游组是否存在
    if let Err(response) = check_upstream_group_exists(&config_write, &payload.target_group) {
        return response;
    }

    // 查找指定的HTTP服务器配置
    let http_server = match get_http_server(&mut config_write) {
        Ok(server) => server,
        Err(response) => return response,
    };

    // 查找指定转发服务的索引
    let forward_index = http_server
        .forwards
        .iter()
        .position(|f| f.name == forward_name);

    match forward_index {
        Some(index) => {
            // 初始化routing字段（如果不存在）
            if http_server.forwards[index].routing.is_none() {
                http_server.forwards[index].routing = Some(Vec::new());
            }

            let routing = http_server.forwards[index].routing.as_mut().unwrap();

            // 检查路径是否已存在
            if routing.iter().any(|r| r.path == payload.path) {
                let error = ErrorResponse::error(
                    StatusCode::CONFLICT,
                    error_types::CONFLICT,
                    format!(
                        "Route with path '{}' already exists in forward '{}'",
                        payload.path, forward_name
                    ),
                );
                log_response_body(&error);
                return (StatusCode::CONFLICT, Json(error)).into_response();
            }

            // 添加新的路由规则
            routing.push(payload.clone());

            info!(
                "API: Created new route '{}' -> '{}' in forward '{}'",
                payload.path, payload.target_group, forward_name
            );

            let response = SuccessResponse::success_with_data(&payload);
            log_response_body(&response);

            (StatusCode::CREATED, success_response(&payload)).into_response()
        }
        None => forward_not_found(&forward_name),
    }
}

/// 更新指定转发服务中的特定路由规则
///
/// Update a specific routing rule in a specified forwarding service
#[utoipa::path(
    put,
    path = "/api/v1/forwards/{name}/routes/{path}",
    tag = "Routes",
    params(
        ("name" = String, Path, description = "转发服务名称"),
        ("path" = String, Path, description = "Base64编码的路径模式")
    ),
    request_body = UpdateRoutePayload,
    responses(
        (status = 200, description = "成功更新路由规则", body = SuccessResponse<RoutingRule>),
        (status = 400, description = "无效的请求参数或Base64编码", body = ErrorResponse),
        (status = 404, description = "转发服务、路由规则或目标上游组不存在", body = ErrorResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse),
    )
)]
pub async fn update_route(
    State(config): State<Arc<RwLock<Config>>>,
    Path((forward_name, encoded_path)): Path<(String, String)>,
    Json(payload): Json<UpdateRoutePayload>,
) -> Response {
    // 记录请求体
    log_request_body(&payload);

    // 验证请求体
    if let Err(e) = payload.validate() {
        let error = ErrorResponse::from_validation_errors(e);
        log_response_body(&error);
        return Json(error).into_response();
    }

    // 解码路径
    let path = match decode_path(&encoded_path) {
        Ok(p) => p,
        Err(response) => return response,
    };

    // 获取配置的写锁
    let mut config_write = config.write().await;

    // 先检查上游组是否存在
    if let Err(response) = check_upstream_group_exists(&config_write, &payload.target_group) {
        return response;
    }

    // 查找指定的HTTP服务器配置
    let http_server = match get_http_server(&mut config_write) {
        Ok(server) => server,
        Err(response) => return response,
    };

    // 查找指定的转发服务
    let forward_index = http_server
        .forwards
        .iter()
        .position(|f| f.name == forward_name);

    match forward_index {
        Some(index) => {
            // 获取路由规则（如果存在）
            let routing = match http_server.forwards[index].routing.as_mut() {
                Some(r) => r,
                None => {
                    let error = ErrorResponse::error(
                        StatusCode::NOT_FOUND,
                        error_types::NOT_FOUND,
                        format!("Forward '{}' has no routing rules", forward_name),
                    );
                    log_response_body(&error);
                    return not_found_error("Route", &path);
                }
            };

            // 查找指定的路由规则
            let route_index = routing.iter().position(|r| r.path == path);

            match route_index {
                Some(idx) => {
                    // 更新路由规则
                    routing[idx].target_group = payload.target_group.clone();

                    info!(
                        "API: Updated route '{}' to target '{}' in forward '{}'",
                        path, payload.target_group, forward_name
                    );

                    let updated_rule = &routing[idx];

                    // 记录响应体
                    let response = SuccessResponse::success_with_data(updated_rule);
                    log_response_body(&response);

                    success_response(updated_rule)
                }
                None => route_not_found(&path, &forward_name),
            }
        }
        None => forward_not_found(&forward_name),
    }
}

/// 删除指定转发服务中的特定路由规则
///
/// Delete a specific routing rule in a specified forwarding service
#[utoipa::path(
    delete,
    path = "/api/v1/forwards/{name}/routes/{path}",
    tag = "Routes",
    params(
        ("name" = String, Path, description = "转发服务名称"),
        ("path" = String, Path, description = "Base64编码的路径模式")
    ),
    responses(
        (status = 204, description = "成功删除路由规则"),
        (status = 400, description = "无效的Base64编码", body = ErrorResponse),
        (status = 404, description = "转发服务或路由规则不存在", body = ErrorResponse),
        (status = 500, description = "服务器内部错误", body = ErrorResponse),
    )
)]
pub async fn delete_route(
    State(config): State<Arc<RwLock<Config>>>,
    Path((forward_name, encoded_path)): Path<(String, String)>,
) -> Response {
    // 解码路径
    let path = match decode_path(&encoded_path) {
        Ok(p) => p,
        Err(response) => return response,
    };

    // 获取配置的写锁
    let mut config_write = config.write().await;

    // 查找指定的HTTP服务器配置
    let http_server = match get_http_server(&mut config_write) {
        Ok(server) => server,
        Err(response) => return response,
    };

    // 查找指定的转发服务
    let forward_index = http_server
        .forwards
        .iter()
        .position(|f| f.name == forward_name);

    match forward_index {
        Some(index) => {
            // 获取路由规则（如果存在）
            let routing = match http_server.forwards[index].routing.as_mut() {
                Some(r) => r,
                None => {
                    let error = ErrorResponse::error(
                        StatusCode::NOT_FOUND,
                        error_types::NOT_FOUND,
                        format!("Forward '{}' has no routing rules", forward_name),
                    );
                    log_response_body(&error);
                    return not_found_error("Route", &path);
                }
            };

            // 查找并删除指定的路由规则
            let initial_len = routing.len();
            routing.retain(|r| r.path != path);

            // 如果长度没有变化，说明没有找到要删除的规则
            if routing.len() == initial_len {
                return route_not_found(&path, &forward_name);
            }

            // 如果删除后路由规则为空，将routing设置为None
            if routing.is_empty() {
                http_server.forwards[index].routing = None;
            }

            info!(
                "API: Deleted route '{}' from forward '{}'",
                path, forward_name
            );

            // 返回204 No Content
            StatusCode::NO_CONTENT.into_response()
        }
        None => forward_not_found(&forward_name),
    }
}
