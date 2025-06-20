use crate::{error::AppError, r#const::http_headers};
use axum::{
    body::{to_bytes, Body},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Router,
};
use socket2::{Domain, Protocol, Socket, Type};
use std::borrow::Cow;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::error;

use super::forward::ForwardState;

/// 检查响应是否为流式响应
///
/// 如果响应是流式响应，则返回 true，否则返回 false
#[inline(always)]
pub(super) fn is_streaming_response(headers: &HeaderMap) -> bool {
    // 检查内容类型是否为事件流
    let is_event_stream = headers
        .get(http_headers::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| s.contains(http_headers::content_types::EVENT_STREAM));

    // 检查传输编码是否为分块
    let is_chunked = headers
        .get(http_headers::TRANSFER_ENCODING)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| s.contains(http_headers::transfer_encodings::CHUNKED));

    // 如果任一条件满足，则认为是流式响应
    is_event_stream || is_chunked
}

/// 标准化请求路径
/// 将请求路径标准化为以斜杠开始的 Cow 字符串
///
/// 如果路径为空或不存在，返回 "/"。
/// 如果路径不以斜杠开头，添加斜杠前缀。
#[inline(always)]
pub(super) fn normalize_path(path: Option<axum::extract::Path<String>>) -> Cow<'static, str> {
    match path {
        Some(p) if !p.0.is_empty() => {
            if p.0.starts_with('/') {
                Cow::Owned(p.0)
            } else {
                Cow::Owned(format!("/{}", p.0))
            }
        }
        _ => Cow::Borrowed("/"),
    }
}

/// 从请求中提取请求体
///
/// 如果提取成功且请求体非空，返回 Some(bytes)，
/// 如果请求体为空，返回 None，
/// 如果提取失败，返回错误信息和状态码
#[inline]
pub(super) async fn extract_request_body(
    body: Body,
    config_name: &str,
) -> Result<Option<bytes::Bytes>, Response> {
    match to_bytes(body, usize::MAX).await {
        Ok(bytes) => {
            if !bytes.is_empty() {
                Ok(Some(bytes))
            } else {
                Ok(None)
            }
        }
        Err(e) => {
            error!("Failed to read request body: {}", e);

            // 记录错误指标
            crate::metrics::METRICS
                .http_request_errors_total()
                .with_label_values(&[
                    config_name,
                    crate::r#const::error_labels::REQUEST_ERROR,
                    "400",
                ])
                .inc();

            Err((
                axum::http::StatusCode::BAD_REQUEST,
                "Unable to read request body",
            )
                .into_response())
        }
    }
}

/// 创建 TCP 监听器
/// 根据提供的地址和监听队列大小创建一个非阻塞的 TCP 监听器。
pub fn create_tcp_listener(addr: SocketAddr, backlog: i32) -> Result<TcpListener, AppError> {
    // 根据地址类型确定域
    let domain = if addr.is_ipv6() {
        Domain::IPV6
    } else {
        Domain::IPV4
    };

    // 创建 socket
    let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))
        .map_err(|e| AppError::Io(Error::new(ErrorKind::Other, e)))?;

    // 设置 SO_REUSEADDR 选项 (所有平台)
    socket
        .set_reuse_address(true)
        .map_err(|e| AppError::Io(Error::new(ErrorKind::Other, e)))?;

    // 在 Linux 平台上设置 SO_REUSEPORT 选项
    #[cfg(target_os = "linux")]
    socket
        .set_reuse_port(true)
        .map_err(|e| AppError::Io(Error::new(ErrorKind::Other, e)))?;

    // 绑定到地址
    let addr = addr.into();
    socket
        .bind(&addr)
        .map_err(|e| AppError::Io(Error::new(ErrorKind::Other, e)))?;

    // 开始监听
    socket
        .listen(backlog)
        .map_err(|e| AppError::Io(Error::new(ErrorKind::Other, e)))?;

    // 设置为非阻塞模式
    socket
        .set_nonblocking(true)
        .map_err(|e| AppError::Io(Error::new(ErrorKind::Other, e)))?;

    // 将 socket2::Socket 转换为 std::net::TcpListener
    let std_listener: std::net::TcpListener = socket.into();

    // 将 std::net::TcpListener 转换为 tokio::net::TcpListener
    TcpListener::from_std(std_listener).map_err(AppError::Io)
}

/// 创建基本路由
pub(super) fn build_router(state: Arc<ForwardState>) -> Router {
    Router::new()
        .route(
            "/{*path}",
            axum::routing::any(super::handler::forward_handler),
        )
        .with_state(state)
}

/// 应用中间件配置
pub(super) fn apply_middlewares(app: Router, state: &Arc<ForwardState>) -> Router {
    let mut app = app;

    // 应用超时配置
    if let Some(timeout_config) = &state.config.timeout {
        // 创建服务构建器和层
        let layers = tower::ServiceBuilder::new()
            // 添加连接超时中间件
            .layer(tower_http::timeout::TimeoutLayer::new(
                std::time::Duration::from_secs(timeout_config.connect),
            ));

        // 应用所有中间件
        app = app.layer(layers.into_inner());
    } else {
        // 使用默认超时配置
        let default_timeout = crate::config::TimeoutConfig::default();
        let layers = tower::ServiceBuilder::new()
            // 添加连接超时中间件
            .layer(tower_http::timeout::TimeoutLayer::new(
                std::time::Duration::from_secs(default_timeout.connect),
            ));

        // 应用所有中间件
        app = app.layer(layers.into_inner());
    }

    // 如果存在限流配置，添加限流中间件
    if let Some(ratelimit_config) = &state.config.ratelimit {
        // 获取转发服务名称，用于指标记录
        let forward_name = state.config.name.clone();

        // 创建限流配置
        let governor_conf = tower_governor::governor::GovernorConfigBuilder::default()
            .per_second(ratelimit_config.per_second as u64)
            .burst_size(ratelimit_config.burst)
            // 添加自定义错误处理，记录限流指标
            .error_handler(move |err: tower_governor::GovernorError| {
                if let tower_governor::GovernorError::TooManyRequests { .. } = err {
                    // 记录限流指标
                    crate::metrics::METRICS
                        .ratelimit_total()
                        .with_label_values(&[&forward_name])
                        .inc();
                }

                let status = match err {
                    tower_governor::GovernorError::TooManyRequests { .. } => {
                        axum::http::StatusCode::TOO_MANY_REQUESTS
                    }
                    _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                };

                status.into_response()
            })
            .finish()
            .unwrap();

        // 创建限流中间件并应用
        app = app.layer(tower_governor::GovernorLayer {
            config: std::sync::Arc::new(governor_conf),
        });
    }

    app
}
