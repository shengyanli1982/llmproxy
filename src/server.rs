use crate::config::ForwardConfig;
use crate::error::AppError;
use crate::metrics::METRICS;
use crate::r#const::{error_labels, http_headers};
use crate::upstream::UpstreamManager;
use axum::{
    body::{to_bytes, Body},
    extract::{Path, Request, State},
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
    Router,
};
use socket2::{Domain, Protocol, Socket, Type};
use std::borrow::Cow;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio_graceful_shutdown::{IntoSubsystem, SubsystemHandle};
use tower_governor::{governor::GovernorConfigBuilder, GovernorError, GovernorLayer};
use tower_http::timeout::TimeoutLayer;
use tracing::{debug, error, info};

// 转发服务状态
pub struct ForwardState {
    // 上游管理器
    upstream_manager: Arc<UpstreamManager>,
    // 转发配置
    config: ForwardConfig,
}

// 转发服务
pub struct ForwardServer {
    // 监听地址
    addr: SocketAddr,
    // 服务状态
    state: Arc<ForwardState>,
}

impl ForwardServer {
    // 创建新的转发服务
    pub fn new(
        config: ForwardConfig,
        upstream_manager: Arc<UpstreamManager>,
    ) -> Result<Self, AppError> {
        // 解析监听地址
        let addr = format!("{}:{}", config.address, config.port)
            .parse()
            .map_err(|e| AppError::Config(format!("Invalid listening address: {}", e)))?;

        let state = Arc::new(ForwardState {
            upstream_manager,
            config,
        });

        Ok(Self { addr, state })
    }

    // 获取服务器监听地址
    pub fn get_addr(&self) -> &SocketAddr {
        &self.addr
    }
}

/// 检查响应是否为流式响应
///
/// 如果响应是流式响应，则返回 true，否则返回 false
#[inline(always)]
fn is_streaming_response(headers: &HeaderMap) -> bool {
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
fn normalize_path(path: Option<Path<String>>) -> Cow<'static, str> {
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
async fn extract_request_body(
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
            METRICS
                .http_request_errors_total()
                .with_label_values(&[config_name, error_labels::REQUEST_ERROR, "400"])
                .inc();

            Err((StatusCode::BAD_REQUEST, "Unable to read request body").into_response())
        }
    }
}

/// 处理上游响应并转换为适合客户端的响应
///
/// 根据响应类型（流式/非流式）处理不同的响应策略
async fn handle_response(
    response: reqwest::Response,
    start_time: Instant,
    config_name: &str,
    method: &Method,
    path: &str,
    upstream_group: &str,
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
    let is_stream = is_streaming_response(&headers);

    // 创建响应构建器
    let mut axum_response = Response::builder().status(status);

    // 复制响应头
    if let Some(headers_mut) = axum_response.headers_mut() {
        *headers_mut = headers;
    }

    // 根据响应类型处理
    let result = if is_stream {
        // 对于流式响应，直接转发流
        debug!("Handling streaming response");

        // 将 reqwest 响应流转换为 axum 流
        let stream = response.bytes_stream();
        // 使用 Body::from_stream 直接传递流，避免额外的内存复制
        let body = Body::from_stream(stream);
        match axum_response.body(body) {
            Ok(response) => response,
            Err(e) => {
                error!("Failed to create streaming response: {}", e);
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
                        error!("Failed to create response: {}", e);
                        StatusCode::INTERNAL_SERVER_ERROR.into_response()
                    }
                }
            }
            Err(e) => {
                error!("Failed to read response body: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    };

    // 记录请求完成的延迟时间（毫秒）
    info!(
        "Request completed: {} {} to upstream group {}, status: {}, time: {}ms",
        method, path, upstream_group, status, duration_ms
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
    upstream_group: &str,
) -> Response {
    error!("Failed to forward request: {}", error);

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
        "Request failed: {} {} to upstream group {}, time: {}ms",
        method,
        path,
        upstream_group,
        duration.as_millis()
    );

    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

/// 创建基本路由
fn build_router(state: Arc<ForwardState>) -> Router {
    Router::new()
        .route("/{*path}", axum::routing::any(forward_handler))
        .with_state(state)
}

/// 应用中间件配置
fn apply_middlewares(app: Router, state: &Arc<ForwardState>) -> Router {
    let mut app = app;

    // 应用超时配置
    let timeout_config = &state.config.timeout;
    // 创建服务构建器和层
    let layers = tower::ServiceBuilder::new()
        // 添加连接超时中间件
        .layer(TimeoutLayer::new(Duration::from_secs(
            timeout_config.connect,
        )));

    // 应用所有中间件
    app = app.layer(layers.into_inner());

    // 如果启用了限流，添加限流中间件
    if state.config.ratelimit.enabled {
        // 获取转发服务名称，用于指标记录
        let forward_name = state.config.name.clone();

        // 创建限流配置
        let governor_conf = GovernorConfigBuilder::default()
            .per_second(state.config.ratelimit.per_second as u64)
            .burst_size(state.config.ratelimit.burst)
            // 添加自定义错误处理，记录限流指标
            .error_handler(move |err: GovernorError| {
                if let GovernorError::TooManyRequests { .. } = err {
                    // 记录限流指标
                    METRICS
                        .ratelimit_total()
                        .with_label_values(&[&forward_name])
                        .inc();
                }

                let status = match err {
                    GovernorError::TooManyRequests { .. } => StatusCode::TOO_MANY_REQUESTS,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                };

                status.into_response()
            })
            .finish()
            .unwrap();

        // 创建限流中间件并应用
        app = app.layer(GovernorLayer {
            config: Arc::new(governor_conf),
        });
    }

    app
}

#[async_trait::async_trait]
impl IntoSubsystem<AppError> for ForwardServer {
    async fn run(self, subsys: SubsystemHandle) -> Result<(), AppError> {
        // 创建路由
        let app = build_router(self.state.clone());

        // 应用中间件
        let app = apply_middlewares(app, &self.state);

        // 创建 TCP 监听器
        let listener = create_tcp_listener(self.addr, u16::MAX.into())?;

        info!(
            "Forwarding service {} listening on {}",
            self.state.config.name, self.addr
        );

        // 使用tokio::select!监听服务器和关闭信号
        tokio::select! {
            result = axum::serve(listener, app) => {
                if let Err(e) = result {
                    error!("Forwarding service error: {}", e);
                } else {
                    info!("Forwarding service completed normally");
                }
                Ok(())
            }
            _ = subsys.on_shutdown_requested() => {
                info!("Shutdown requested, stopping forwarding service");
                Ok(())
            }
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

    // 转发请求
    match state
        .upstream_manager
        .forward_request(
            &state.config.upstream_group,
            &method,
            &path,
            headers,
            body_bytes,
        )
        .await
    {
        Ok(response) => {
            handle_response(
                response,
                start_time,
                &state.config.name,
                &method,
                &path,
                &state.config.upstream_group,
            )
            .await
        }
        Err(e) => handle_request_error(
            &e,
            start_time,
            &state.config.name,
            &method,
            &path,
            &state.config.upstream_group,
        ),
    }
}
