use crate::{
    balancer::{create_load_balancer, LoadBalancer, ManagedUpstream},
    breaker::{create_upstream_circuit_breaker, UpstreamError},
    config::{
        AuthConfig, AuthType, HeaderOpType, HttpClientConfig, UpstreamConfig, UpstreamGroupConfig,
    },
    error::AppError,
    metrics::METRICS,
    r#const::{
        balance_strategy_labels, breaker_result_labels, error_labels, retry_limits, upstream_labels,
    },
};
use reqwest::{header::HeaderMap, Method, Response, Url};
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use retry_policies::Jitter;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{debug, error, info};

// 上游管理器
pub struct UpstreamManager {
    // 上游配置映射
    upstreams: HashMap<String, UpstreamConfig>,
    // 上游组负载均衡器
    groups: HashMap<String, Arc<dyn LoadBalancer>>,
    // 上游组客户端
    group_clients: HashMap<String, ClientWithMiddleware>,
}

impl UpstreamManager {
    // 创建新的上游管理器
    pub async fn new(
        upstreams: Vec<UpstreamConfig>,
        groups: Vec<UpstreamGroupConfig>,
    ) -> Result<Self, AppError> {
        let mut upstream_map = HashMap::with_capacity(upstreams.len());
        let mut group_map = HashMap::with_capacity(groups.len());
        let mut group_clients = HashMap::with_capacity(groups.len());

        // 构建上游映射
        for upstream in upstreams {
            debug!(
                "Loaded upstream: {}, id: {}, url: {:?}",
                upstream.name, upstream.id, upstream.url
            );
            upstream_map.insert(upstream.name.clone(), upstream);
        }

        // 为每个组创建负载均衡器和HTTP客户端
        for group in groups {
            let group_name = &group.name;
            // 获取组内所有上游的引用
            let upstream_refs = &group.upstreams;

            // 创建托管上游列表
            let mut managed_upstreams = Vec::with_capacity(upstream_refs.len());

            for upstream_ref in upstream_refs {
                // 获取完整的上游配置
                let upstream_config = match upstream_map.get(&upstream_ref.name) {
                    Some(config) => config,
                    None => {
                        return Err(AppError::Config(format!(
                            "Referenced upstream '{}' not found in upstreams configuration",
                            upstream_ref.name
                        )));
                    }
                };

                // 创建熔断器（如果上游配置了熔断器）
                let breaker = match &upstream_config.breaker {
                    Some(breaker_config) => {
                        let breaker = create_upstream_circuit_breaker(
                            upstream_config.id.clone(),
                            upstream_ref.name.clone(),
                            group_name.clone(),
                            upstream_config.url.clone(),
                            breaker_config,
                        );
                        Some(breaker)
                    }
                    None => None,
                };

                // 创建托管上游
                let managed_upstream = ManagedUpstream {
                    upstream_ref: upstream_ref.clone(),
                    id: upstream_config.id.clone(),
                    breaker,
                };

                managed_upstreams.push(managed_upstream);
            }

            // 创建负载均衡器
            let lb = create_load_balancer(&group.balance.strategy, managed_upstreams);

            // 创建该组的HTTP客户端
            let client = Self::create_http_client(&group.http_client)?;
            group_clients.insert(group.name.clone(), client);

            group_map.insert(group.name.clone(), lb);
        }

        info!("Initialized {} upstream groups", group_map.len());

        Ok(Self {
            upstreams: upstream_map,
            groups: group_map,
            group_clients,
        })
    }

    // 创建HTTP客户端
    fn create_http_client(config: &HttpClientConfig) -> Result<ClientWithMiddleware, AppError> {
        debug!("Creating HTTP client, config: {:?}", config);

        // 创建客户端构建器
        let mut client_builder = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(true) // 允许无效证书，用于内部自签名证书
            .connect_timeout(Duration::from_secs(config.timeout.connect));

        // 仅为非流式响应设置请求超时
        if !config.stream_mode {
            client_builder = client_builder.timeout(Duration::from_secs(config.timeout.request));
        }

        // 配置TCP keepalive（如果启用）
        if config.keepalive > 0 {
            client_builder =
                client_builder.tcp_keepalive(Duration::from_secs(config.keepalive as u64));
        }

        // 配置空闲连接超时（如果设置）
        if config.timeout.idle > 0 {
            client_builder =
                client_builder.pool_idle_timeout(Duration::from_secs(config.timeout.idle));
        }

        // 配置用户代理（如果有）
        if !config.agent.is_empty() {
            client_builder = client_builder.user_agent(&config.agent);
        }

        // 配置代理（如果启用）
        if config.proxy.enabled && !config.proxy.url.is_empty() {
            client_builder =
                client_builder.proxy(reqwest::Proxy::all(&config.proxy.url).map_err(|e| {
                    AppError::InvalidProxy(format!("Proxy configuration error: {}", e))
                })?);
        }

        // 创建基础HTTP客户端
        let client = client_builder.build().map_err(AppError::HttpError)?;

        // 配置重试策略（根据组的重试配置）
        let middleware_client = if config.retry.enabled {
            // 使用指数退避策略，基于组的重试配置
            let retry_policy = ExponentialBackoff::builder()
                .retry_bounds(
                    Duration::from_millis(config.retry.initial.into()),
                    Duration::from_secs(retry_limits::MAX_DELAY.into()),
                )
                .base(2)
                .jitter(Jitter::Bounded)
                .build_with_max_retries(config.retry.attempts);

            reqwest_middleware::ClientBuilder::new(client)
                .with(RetryTransientMiddleware::new_with_policy(retry_policy))
                .build()
        } else {
            // 不进行重试
            reqwest_middleware::ClientBuilder::new(client).build()
        };

        Ok(middleware_client)
    }

    // 转发请求到指定上游组
    pub async fn forward_request(
        &self,
        group_name: &str,
        method: Method,
        path: &str,
        headers: HeaderMap,
        body: Option<bytes::Bytes>,
    ) -> Result<Response, AppError> {
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
        let managed_upstream = match load_balancer.select_upstream().await {
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
        let upstream_config = match self.upstreams.get(&managed_upstream.upstream_ref.name) {
            Some(config) => config,
            None => {
                error!(
                    "Upstream configuration not found: {}",
                    managed_upstream.upstream_ref.name
                );
                return Err(AppError::Upstream(format!(
                    "Upstream configuration not found: {}",
                    managed_upstream.upstream_ref.name
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

        // 构建请求URL - 使用 String::with_capacity 预分配内存
        let url_capacity = upstream_config.url.len() + path.len();
        let mut url = String::with_capacity(url_capacity);
        url.push_str(&upstream_config.url);
        url.push_str(path);

        let url_parsed = Url::parse(&url)
            .map_err(|e| AppError::Upstream(format!("Invalid upstream URL: {} - {}", url, e)))?;

        // 获取组的HTTP客户端
        let client = match self.group_clients.get(group_name) {
            Some(c) => c,
            None => {
                error!("HTTP client not found: {}", group_name);
                return Err(AppError::UpstreamGroupNotFound(group_name.to_string()));
            }
        };

        // 定义请求执行闭包 - 使用引用捕获以减少克隆
        let upstream_url = &upstream_config.url;
        let request_future = |headers: HeaderMap, body: Option<bytes::Bytes>| {
            let url_parsed = url_parsed.clone();
            let method = method.clone();
            let upstream_url = upstream_url.clone();
            let client = client.clone();

            async move {
                // 创建请求构建器
                let mut request_builder = client.request(method, url_parsed);

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
                    Ok(response) => Ok(response),
                    Err(e) => Err(UpstreamError(format!(
                        "Request to {} failed: {}",
                        upstream_url.as_str(),
                        e
                    ))),
                }
            }
        };

        // 执行请求（根据是否有熔断器决定执行方式）
        let response = match &managed_upstream.breaker {
            Some(breaker) => {
                // 使用熔断器执行请求
                match breaker
                    .call_async(move || request_future(headers, body))
                    .await
                {
                    Ok(resp) => Ok(resp),
                    Err(circuitbreaker_rs::BreakerError::Open) => {
                        // 熔断器开启，拒绝请求
                        error!(
                            "Circuit breaker is open for upstream: {}",
                            upstream_config.url.as_str()
                        );

                        // 记录被拒绝的请求
                        METRICS
                            .circuitbreaker_calls_total()
                            .with_label_values(&[
                                group_name,
                                &managed_upstream.upstream_ref.name,
                                &upstream_config.url,
                                breaker_result_labels::REJECTED,
                            ])
                            .inc();

                        Err(AppError::CircuitBreakerOpen(upstream_config.url.clone()))
                    }
                    Err(circuitbreaker_rs::BreakerError::Operation(op_err)) => {
                        // 请求执行失败
                        error!("Operation error: {}", op_err);
                        Err(AppError::Upstream(op_err.0))
                    }
                    Err(e) => {
                        // 其他熔断器错误
                        error!("Circuit breaker error: {}", e);
                        Err(AppError::Upstream(format!("Circuit breaker error: {}", e)))
                    }
                }
            }
            None => {
                // 直接执行请求（无熔断器保护）
                request_future(headers, body)
                    .await
                    .map_err(|err| AppError::Upstream(err.0))
            }
        };

        // 记录上游请求耗时
        let duration = start_time.elapsed();
        METRICS
            .upstream_duration_seconds()
            .with_label_values(&[group_name, &upstream_config.url])
            .observe(duration.as_secs_f64());

        // 检查是否为响应时间感知的负载均衡器，更新指标
        if load_balancer.as_str() == balance_strategy_labels::RESPONSE_AWARE {
            let duration_ms = duration.as_millis() as usize;
            if let Some(response_aware) = load_balancer
                .as_any()
                .downcast_ref::<crate::balancer::ResponseAwareBalancer>()
            {
                response_aware.update_metrics(managed_upstream, duration_ms);
            }
        }

        // 错误处理和指标记录
        if let Err(ref err) = response {
            // 报告上游失败
            load_balancer.report_failure(managed_upstream).await;

            // 记录错误指标
            let error_label = match err {
                AppError::CircuitBreakerOpen(_) => "circuit_open",
                AppError::Upstream(_) => "upstream_error",
                _ => "other_error",
            };

            METRICS.record_upstream_request_error(group_name, &upstream_config.url, error_label);
        } else if let Ok(ref response) = response {
            // 记录响应状态码
            let status = response.status().as_u16();
            debug!(
                "Upstream response status: {} from {}",
                status,
                upstream_config.url.as_str()
            );
        }

        response
    }

    // 处理请求头
    fn process_headers(
        &self,
        headers: HeaderMap,
        upstream: &UpstreamConfig,
    ) -> Result<HeaderMap, AppError> {
        // 创建新的 HeaderMap 而不是克隆
        let mut result = HeaderMap::with_capacity(headers.len());

        // 先复制所有原始头
        for (key, value) in headers.iter() {
            result.insert(key, value.clone());
        }

        // 处理请求头操作
        for op in &upstream.headers {
            match op.op {
                HeaderOpType::Insert | HeaderOpType::Replace => {
                    if let (Some(name), Some(value)) = (&op.parsed_name, &op.parsed_value) {
                        result.insert(name.clone(), value.clone());
                    }
                }
                HeaderOpType::Remove => {
                    if let Some(name) = &op.parsed_name {
                        result.remove(name);
                    }
                }
            }
        }

        Ok(result)
    }

    // 添加认证信息到请求
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
