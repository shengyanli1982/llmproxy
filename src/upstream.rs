use crate::{
    balancer::{create_load_balancer, LoadBalancer},
    config::{
        AuthConfig, AuthType, HeaderOpType, HttpClientConfig, UpstreamConfig, UpstreamGroupConfig,
    },
    error::AppError,
    metrics::METRICS,
    r#const::{error_labels, retry_limits, upstream_labels},
};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Method, Url,
};
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use retry_policies::Jitter;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{debug, error, info};

/// 上游管理器
pub struct UpstreamManager {
    /// 上游配置映射
    upstreams: HashMap<String, UpstreamConfig>,
    /// 上游组负载均衡器
    groups: HashMap<String, Arc<dyn LoadBalancer>>,
    /// 上游组客户端
    group_clients: HashMap<String, ClientWithMiddleware>,
}

impl UpstreamManager {
    /// 创建新的上游管理器
    pub async fn new(
        upstreams: Vec<UpstreamConfig>,
        groups: Vec<UpstreamGroupConfig>,
    ) -> Result<Self, AppError> {
        let mut upstream_map = HashMap::with_capacity(upstreams.len());
        let mut group_map = HashMap::with_capacity(groups.len());
        let mut group_clients = HashMap::new();

        // 构建上游映射
        for upstream in upstreams {
            upstream_map.insert(upstream.name.clone(), upstream);
        }

        // 为每个组创建负载均衡器和HTTP客户端
        for group in groups {
            // 获取组内所有上游的引用
            let group_upstreams = group.upstreams.clone();

            // 创建负载均衡器
            let lb = create_load_balancer(&group.balance.strategy, group_upstreams);

            // 创建该组的HTTP客户端
            let client = Self::create_http_client(&group.http_client)?;
            group_clients.insert(group.name.clone(), client);

            group_map.insert(group.name, lb);
        }

        info!("Initialized {} upstream groups", group_map.len());

        Ok(Self {
            upstreams: upstream_map,
            groups: group_map,
            group_clients,
        })
    }

    /// 创建HTTP客户端
    fn create_http_client(config: &HttpClientConfig) -> Result<ClientWithMiddleware, AppError> {
        debug!("Creating HTTP client, config: {:?}", config);

        // 创建客户端构建器
        let mut client_builder = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(true) // 允许无效证书，用于内部自签名证书
            .connect_timeout(Duration::from_secs(config.timeout.connect))
            .timeout(Duration::from_secs(config.timeout.request));

        // 配置TCP keepalive
        if config.keepalive > 0 {
            client_builder =
                client_builder.tcp_keepalive(Duration::from_secs(config.keepalive as u64));
        }

        // 配置空闲连接超时
        if config.timeout.idle > 0 {
            client_builder =
                client_builder.pool_idle_timeout(Duration::from_secs(config.timeout.idle));
        }

        // 配置用户代理
        if !config.agent.is_empty() {
            client_builder = client_builder.user_agent(&config.agent);
        }

        // 配置代理
        if config.proxy.enabled && !config.proxy.url.is_empty() {
            client_builder =
                client_builder.proxy(reqwest::Proxy::all(&config.proxy.url).map_err(|e| {
                    AppError::InvalidProxy(format!("Proxy configuration error: {}", e))
                })?);
        }

        // 创建基础HTTP客户端
        let client = client_builder.build().map_err(|e| AppError::HttpError(e))?;

        // 配置重试策略（根据组的重试配置）
        let middleware_client = if config.retry.enabled {
            // 使用指数退避策略，基于组的重试配置
            let retry_policy = ExponentialBackoff::builder()
                .retry_bounds(
                    Duration::from_millis(config.retry.initial.into()),
                    Duration::from_secs(retry_limits::MAX_DELAY.into()),
                )
                // 设置延迟基值
                .base(2)
                // 使用有界抖动来避免多个客户端同时重试
                .jitter(Jitter::Bounded)
                // 配置最大重试次数
                .build_with_max_retries(config.retry.attempts as u32);

            reqwest_middleware::ClientBuilder::new(client)
                .with(RetryTransientMiddleware::new_with_policy(retry_policy))
                .build()
        } else {
            // 不进行重试
            reqwest_middleware::ClientBuilder::new(client).build()
        };

        Ok(middleware_client)
    }

    /// 转发请求到指定上游组
    pub async fn forward_request(
        &self,
        group_name: &str,
        method: Method,
        path: &str,
        headers: HeaderMap,
        body: Option<bytes::Bytes>,
    ) -> Result<reqwest::Response, AppError> {
        debug!("Forwarding request to upstream group: {}", group_name);

        // 获取上游组的负载均衡器
        let load_balancer = match self.groups.get(group_name) {
            Some(lb) => lb,
            None => {
                error!("Upstream group not found: {}", group_name);
                return Err(AppError::UpstreamGroupNotFound(group_name.to_string()));
            }
        };

        // 选择一个上游服务器
        let upstream_ref = match load_balancer.select_upstream().await {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to select upstream server: {}", e);

                // 记录上游错误指标
                METRICS
                    .upstream_errors_total()
                    .with_label_values(&[
                        error_labels::SELECT_ERROR,
                        group_name,
                        upstream_labels::UNKNOWN,
                    ])
                    .inc();

                return Err(e);
            }
        };

        // 获取上游配置
        let upstream_config = match self.upstreams.get(&upstream_ref.name) {
            Some(config) => config,
            None => {
                error!("Upstream configuration not found: {}", upstream_ref.name);
                return Err(AppError::Upstream(format!(
                    "Upstream configuration not found: {}",
                    upstream_ref.name
                )));
            }
        };

        debug!("Selected upstream server: {}", upstream_config.url);

        // 记录上游请求指标
        METRICS
            .upstream_requests_total()
            .with_label_values(&[group_name, &upstream_config.url])
            .inc();

        // 记录开始时间
        let start_time = Instant::now();

        // 构建请求URL
        let url = format!("{}{}", upstream_config.url, path);
        let url = Url::parse(&url)
            .map_err(|e| AppError::Upstream(format!("Invalid upstream URL: {} - {}", url, e)))?;

        // 获取组的HTTP客户端
        let client = match self.group_clients.get(group_name) {
            Some(c) => c,
            None => {
                error!("HTTP client not found: {}", group_name);
                return Err(AppError::UpstreamGroupNotFound(group_name.to_string()));
            }
        };

        // 创建请求构建器
        let mut request_builder = client.request(method, url);

        // 处理请求头
        let processed_headers = self.process_headers(headers, upstream_config)?;
        request_builder = request_builder.headers(processed_headers);

        // 添加认证信息
        if let Some(ref auth) = upstream_config.auth {
            request_builder = self.add_auth(request_builder, auth)?;
        }

        // 添加请求体（如果有）
        if let Some(body_data) = body {
            request_builder = request_builder.body(body_data);
        }

        // 发送请求
        match request_builder.send().await {
            Ok(response) => {
                // 记录上游请求耗时
                let duration = start_time.elapsed();
                METRICS
                    .upstream_duration_seconds()
                    .with_label_values(&[group_name, &upstream_config.url])
                    .observe(duration.as_secs_f64());

                Ok(response)
            }
            Err(e) => {
                error!("Upstream request failed: {} - {}", upstream_config.url, e);

                // 记录上游请求耗时（即使失败也记录）
                let duration = start_time.elapsed();
                METRICS
                    .upstream_duration_seconds()
                    .with_label_values(&[group_name, &upstream_config.url])
                    .observe(duration.as_secs_f64());

                // 报告上游失败
                load_balancer.report_failure(upstream_ref).await;

                // 记录上游错误指标
                METRICS
                    .upstream_errors_total()
                    .with_label_values(&[
                        error_labels::REQUEST_ERROR,
                        group_name,
                        &upstream_config.url,
                    ])
                    .inc();

                Err(AppError::HttpMiddlewareError(e))
            }
        }
    }

    /// 处理请求头
    fn process_headers(
        &self,
        headers: HeaderMap,
        upstream: &UpstreamConfig,
    ) -> Result<HeaderMap, AppError> {
        let mut result = headers.clone();

        // 处理请求头操作
        for op in &upstream.headers {
            match op.op {
                HeaderOpType::Insert | HeaderOpType::Replace => {
                    let header_name = HeaderName::from_bytes(op.key.as_bytes()).map_err(|e| {
                        AppError::InvalidHeader(format!("Invalid header name: {}", e))
                    })?;
                    let value_str = op.value.as_deref().unwrap_or_default();
                    let header_value = HeaderValue::from_str(value_str).map_err(|e| {
                        AppError::InvalidHeader(format!("Invalid header value: {}", e))
                    })?;
                    result.insert(header_name, header_value);
                }
                HeaderOpType::Remove => {
                    let header_name = HeaderName::from_bytes(op.key.as_bytes()).map_err(|e| {
                        AppError::InvalidHeader(format!("Invalid header name: {}", e))
                    })?;
                    result.remove(header_name);
                }
            }
        }

        Ok(result)
    }

    /// 添加认证信息到请求
    fn add_auth(
        &self,
        request: reqwest_middleware::RequestBuilder,
        auth: &AuthConfig,
    ) -> Result<reqwest_middleware::RequestBuilder, AppError> {
        match auth.r#type {
            AuthType::Basic => {
                if let (Some(username), Some(password)) = (&auth.username, &auth.password) {
                    Ok(request.basic_auth(username, Some(password)))
                } else {
                    Err(AppError::AuthError("Basic auth config missing".to_string()))
                }
            }
            AuthType::Bearer => {
                if let Some(token) = &auth.token {
                    Ok(request.bearer_auth(token))
                } else {
                    Err(AppError::AuthError("Bearer auth token missing".to_string()))
                }
            }
            AuthType::None => Ok(request),
        }
    }
}
