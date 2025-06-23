use axum::{
    body::Body,
    extract::{Path, Request, State},
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info};

use crate::{error::AppError, metrics::METRICS, r#const::error_labels};

use super::{
    forward::ForwardState,
    utils::{extract_request_body, normalize_path},
};

/// 处理上游响应并转换为适合客户端的响应
///
/// 根据响应类型（流式/非流式）处理不同的响应策略
async fn handle_response(
    response: reqwest::Response,
    start_time: Instant,
    config_name: &str,
    method: &Method,
    path: &str,
    default_group: &str,
) -> Response {
    // 获取响应状态码和头
    let status = response.status();
    let headers = response.headers().clone();

    // 记录请求耗时
    let duration = start_time.elapsed();
    let duration_ms = duration.as_millis();

    METRICS
        .http_request_duration_seconds()
        .with_label_values(&[config_name, method.as_str()])
        .observe(duration.as_secs_f64());

    // 如果状态码表示错误，记录错误指标
    if status.is_client_error() || status.is_server_error() {
        METRICS
            .http_request_errors_total()
            .with_label_values(&[
                config_name,
                error_labels::UPSTREAM_ERROR,
                &status.as_u16().to_string(),
            ])
            .inc();
    }

    // 检查是否为流式响应
    let is_stream = super::utils::is_streaming_response(&headers);

    // 创建响应构建器
    let mut axum_response = Response::builder().status(status);

    // 复制响应头
    if let Some(headers_mut) = axum_response.headers_mut() {
        *headers_mut = headers;
    }

    // 根据响应类型处理
    let result = if is_stream {
        // 对于流式响应，直接转发流
        tracing::debug!("Handling streaming response");

        // 将 reqwest 响应流转换为 axum 流
        let stream = response.bytes_stream();
        // 使用 Body::from_stream 直接传递流，避免额外的内存复制
        let body = Body::from_stream(stream);
        match axum_response.body(body) {
            Ok(response) => response,
            Err(e) => {
                tracing::error!("Failed to create streaming response: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    } else {
        // 对于非流式响应，读取完整响应体
        match response.bytes().await {
            Ok(bytes) => {
                // 直接使用 bytes 构建响应体，避免额外的内存复制
                match axum_response.body(Body::from(bytes)) {
                    Ok(response) => response,
                    Err(e) => {
                        tracing::error!("Failed to create response: {}", e);
                        StatusCode::INTERNAL_SERVER_ERROR.into_response()
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to read response body: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    };

    // 记录请求完成的延迟时间（毫秒）
    info!(
        "Request completed: {:?} {:?} to upstream group {:?}, status: {}, time: {}ms",
        method, path, default_group, status, duration_ms
    );

    result
}

/// 处理请求错误并生成适当的错误响应
fn handle_request_error(
    error: &AppError,
    start_time: Instant,
    config_name: &str,
    method: &Method,
    path: &str,
    default_group: &str,
) -> Response {
    tracing::error!("Failed to forward request: {}", error);

    // 记录错误指标
    METRICS
        .http_request_errors_total()
        .with_label_values(&[config_name, error_labels::UPSTREAM_ERROR, "500"])
        .inc();

    // 记录请求耗时
    let duration = start_time.elapsed();
    METRICS
        .http_request_duration_seconds()
        .with_label_values(&[config_name, method.as_str()])
        .observe(duration.as_secs_f64());

    // 记录请求失败的信息
    info!(
        "Request failed: {:?} {:?} to upstream group {:?}, time: {}ms",
        method,
        path,
        default_group,
        duration.as_millis()
    );

    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

// 转发处理函数
pub async fn forward_handler(
    State(state): State<Arc<ForwardState>>,
    path: Option<Path<String>>,
    method: Method,
    headers: HeaderMap,
    req: Request<Body>,
) -> Response {
    // 记录开始时间
    let start_time = Instant::now();

    // 标准化请求路径
    let path = normalize_path(path);
    debug!("Forwarding request url path: {:?}", path);

    // 记录请求指标
    METRICS
        .http_requests_total()
        .with_label_values(&[&state.config.name, method.as_str()])
        .inc();

    // 提取请求体
    let (_, body) = req.into_parts();
    let body_bytes = match extract_request_body(body, &state.config.name).await {
        Ok(bytes) => bytes,
        Err(response) => return response,
    };

    // 此处应该还有一个路由模块
    // 可以根据用户的请求路径，来选择不同的上游组
    //
    // 输入: 请求路径
    // 输出: 上游组名称
    //
    // 1. 根据请求路径，找到对应的 routing 规则
    // 2. 如果找到对应的 routing 规则，则使用对应的 "target_group", 同时 "target_group" 必须在 "upstream_groups" 中定义, 如果 "target_group" 没有定义, 则使用默认的 "default_group" 配置。
    // 3. 如果找不到对应的 routing 规则，则使用默认的 "default_group" 配置。
    //
    // 使用路由器获取目标上游组
    let routing_result = state.router.get_target_group(&path).await;
    let target_group = &routing_result.target_group;

    // 记录路由匹配
    METRICS.record_route_match(&state.config.name, target_group);

    // 转发请求
    match state
        .upstream_manager
        .forward_request(target_group, &method, headers, body_bytes)
        .await
    {
        Ok(response) => {
            handle_response(
                response,
                start_time,
                &state.config.name,
                &method,
                &path,
                target_group,
            )
            .await
        }
        Err(e) => handle_request_error(
            &e,
            start_time,
            &state.config.name,
            &method,
            &path,
            target_group,
        ),
    }
}
