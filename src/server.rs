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

// 检查响应是否为流式响应
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

#[async_trait::async_trait]
impl IntoSubsystem<AppError> for ForwardServer {
    async fn run(self, subsys: SubsystemHandle) -> Result<(), AppError> {
        // 创建路由
        let app = Router::new()
            .route("/{*path}", axum::routing::any(forward_handler))
            .with_state(self.state.clone());

        // 应用超时配置
        let timeout_config = &self.state.config.timeout;

        // 创建服务构建器和层
        let layers = tower::ServiceBuilder::new()
            // 添加连接超时中间件
            .layer(TimeoutLayer::new(Duration::from_secs(
                timeout_config.connect,
            )));

        // 应用所有中间件
        let mut app = app.layer(layers.into_inner());

        // 如果启用了限流，添加限流中间件
        if self.state.config.ratelimit.enabled {
            // 获取转发服务名称，用于指标记录
            let forward_name = self.state.config.name.clone();

            // 创建限流配置
            let governor_conf = GovernorConfigBuilder::default()
                .per_second(self.state.config.ratelimit.per_second as u64)
                .burst_size(self.state.config.ratelimit.burst)
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

        // 绑定TCP监听器
        let listener = match TcpListener::bind(self.addr).await {
            Ok(listener) => {
                info!(
                    "Forwarding service {} listening on {}",
                    self.state.config.name, self.addr
                );
                listener
            }
            Err(e) => {
                error!("Failed to bind forwarding service: {}", e);
                return Err(AppError::Io(e));
            }
        };

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

    // 获取请求路径 - 避免不必要的字符串分配
    let path_str = match &path {
        Some(p) => {
            let path_value = &p.0;
            if path_value.is_empty() {
                "/".to_string()
            } else {
                // 只有在路径不是以 / 开头时才添加
                if path_value.starts_with('/') {
                    path_value.to_string()
                } else {
                    // 在这种情况下才需要分配新字符串
                    let mut path_with_slash = String::with_capacity(path_value.len() + 1);
                    path_with_slash.push('/');
                    path_with_slash.push_str(path_value);
                    path_with_slash
                }
            }
        }
        None => "/".to_string(),
    };

    // 记录请求指标
    METRICS
        .http_requests_total()
        .with_label_values(&[&state.config.name, method.as_str(), &path_str])
        .inc();

    // 提取请求体 - 使用更高效的方式处理请求体
    let (_, body) = req.into_parts();
    let body_bytes = match to_bytes(body, usize::MAX).await {
        Ok(bytes) => {
            if !bytes.is_empty() {
                Some(bytes)
            } else {
                None
            }
        }
        Err(e) => {
            error!("Failed to read request body: {}", e);

            // 记录错误指标
            METRICS
                .http_request_errors_total()
                .with_label_values(&[&state.config.name, error_labels::REQUEST_ERROR, "400"])
                .inc();

            return (StatusCode::BAD_REQUEST, "Unable to read request body").into_response();
        }
    };

    // 转发请求
    match state
        .upstream_manager
        .forward_request(
            &state.config.upstream_group,
            method.clone(),
            &path_str,
            headers,
            body_bytes,
        )
        .await
    {
        Ok(response) => {
            // 获取响应状态码
            let status = response.status();

            // 记录请求耗时
            let duration = start_time.elapsed();
            let duration_ms = duration.as_millis();

            METRICS
                .http_request_duration_seconds()
                .with_label_values(&[&state.config.name, method.as_str(), &path_str])
                .observe(duration.as_secs_f64());

            // 如果状态码表示错误，记录错误指标
            if status.is_client_error() || status.is_server_error() {
                METRICS
                    .http_request_errors_total()
                    .with_label_values(&[
                        &state.config.name,
                        error_labels::UPSTREAM_ERROR,
                        &status.as_u16().to_string(),
                    ])
                    .inc();
            }

            // 检查是否为流式响应
            let is_stream = is_streaming_response(response.headers());

            // 创建响应构建器
            let mut axum_response = Response::builder().status(status);

            // 复制响应头
            if let Some(headers_mut) = axum_response.headers_mut() {
                // 直接移动头，而不是克隆
                for (name, value) in response.headers() {
                    headers_mut.insert(name.clone(), value.clone());
                }
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
                method, path_str, state.config.upstream_group, status, duration_ms
            );

            result
        }
        Err(e) => {
            error!("Failed to forward request: {}", e);

            // 记录错误指标
            METRICS
                .http_request_errors_total()
                .with_label_values(&[&state.config.name, error_labels::UPSTREAM_ERROR, "500"])
                .inc();

            // 记录请求耗时
            let duration = start_time.elapsed();
            METRICS
                .http_request_duration_seconds()
                .with_label_values(&[&state.config.name, method.as_str(), &path_str])
                .observe(duration.as_secs_f64());

            // 记录请求失败的信息
            info!(
                "Request failed: {} {} to upstream group {}, time: {}ms",
                method,
                path_str,
                state.config.upstream_group,
                duration.as_millis()
            );

            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
