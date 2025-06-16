use crate::{
    balancer::LoadBalancer,
    breaker::UpstreamError,
    config::{HeaderOpType, UpstreamConfig, UpstreamGroupConfig},
    error::AppError,
    metrics::METRICS,
    r#const::{balance_strategy_labels, breaker_result_labels, error_labels, upstream_labels},
};
use bytes::Bytes;
use reqwest::{header::HeaderMap, Method, Response, Url};
use reqwest_middleware::ClientWithMiddleware;
use std::{
    collections::HashMap,
    future::Future,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{debug, error, info, warn};

use super::{
    builder::{build_upstream_map, create_managed_upstream},
    http_client::{add_auth, create_group_clients},
};

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
        let upstream_map = build_upstream_map(&upstreams);
        let mut group_map = HashMap::with_capacity(groups.len());
        let group_clients = create_group_clients(&groups)?;

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

                // 创建托管上游
                let managed_upstream =
                    create_managed_upstream(upstream_ref, upstream_config, group_name)?;

                managed_upstreams.push(managed_upstream);
            }

            // 创建负载均衡器
            let lb =
                crate::balancer::create_load_balancer(&group.balance.strategy, managed_upstreams);

            group_map.insert(group.name.clone(), lb);
        }

        info!("Initialized {} upstream groups", group_map.len());

        Ok(Self {
            upstreams: upstream_map,
            groups: group_map,
            group_clients,
        })
    }

    /// 构建请求URL
    fn build_request_url(&self, upstream_url: &str, path: &str) -> Result<Url, AppError> {
        // 构建请求URL - 使用 String::with_capacity 预分配内存
        let url_capacity = upstream_url.len() + path.len();
        let mut url = String::with_capacity(url_capacity);
        url.push_str(upstream_url);
        url.push_str(path);

        Url::parse(&url)
            .map_err(|e| AppError::Upstream(format!("Invalid upstream URL: {} - {}", url, e)))
    }

    /// 从上游组中选择上游服务器并获取其配置
    async fn select_upstream_server(
        &self,
        group_name: &str,
    ) -> Result<(crate::balancer::ManagedUpstream, &UpstreamConfig), AppError> {
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

        Ok((managed_upstream.clone(), upstream_config))
    }

    /// 执行HTTP请求
    async fn execute_request(
        &self,
        managed_upstream: &crate::balancer::ManagedUpstream,
        upstream_url: &str,
        request_future: impl Future<Output = Result<Response, UpstreamError>> + Send,
        group_name: &str,
    ) -> Result<Response, AppError> {
        // 执行请求（根据是否有熔断器决定执行方式）
        match &managed_upstream.breaker {
            Some(breaker) => {
                // 使用熔断器执行请求
                match breaker.call_async(|| request_future).await {
                    Ok(resp) => Ok(resp),
                    Err(circuitbreaker_rs::BreakerError::Open) => {
                        // 熔断器开启，拒绝请求
                        error!("Circuit breaker is open for upstream: {}", upstream_url);

                        // 记录被拒绝的请求
                        METRICS
                            .circuitbreaker_calls_total()
                            .with_label_values(&[
                                group_name,
                                &managed_upstream.upstream_ref.name,
                                upstream_url,
                                breaker_result_labels::REJECTED,
                            ])
                            .inc();

                        Err(AppError::CircuitBreakerOpen(
                            upstream_url.to_string().into(),
                        ))
                    }
                    Err(circuitbreaker_rs::BreakerError::Operation(op_err)) => {
                        // 请求执行失败
                        error!("Operation error: {}", op_err);
                        Err(AppError::Upstream(op_err.0))
                    }
                    Err(e) => {
                        // 其他熔断器错误
                        error!("Circuit breaker error: {}", e);
                        Err(e.into())
                    }
                }
            }
            None => {
                // 直接执行请求（无熔断器保护）
                request_future
                    .await
                    .map_err(|err| AppError::Upstream(err.0))
            }
        }
    }

    /// 更新响应时间感知的负载均衡器的指标
    fn update_balancer_metrics(
        &self,
        load_balancer: &Arc<dyn LoadBalancer>,
        managed_upstream: &crate::balancer::ManagedUpstream,
        duration: Duration,
    ) {
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
    }

    // 转发请求到指定上游组
    pub async fn forward_request(
        &self,
        group_name: &str,
        method: &Method,
        path: &str,
        headers: HeaderMap,
        body: Option<Bytes>,
    ) -> Result<Response, AppError> {
        debug!("Forwarding request to upstream group: {}", group_name);

        // 选择一个上游服务器
        let (managed_upstream, upstream_config) = self.select_upstream_server(group_name).await?;

        // 记录开始时间
        let start_time = Instant::now();

        // 构建请求URL
        let url = self.build_request_url(&upstream_config.url, path)?;

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
        let request_future = |headers: HeaderMap, body: Option<Bytes>| {
            let url = url.clone();
            let method = method.clone(); // 使用引用的方法，克隆更轻量
            let client = client.clone();

            async move {
                // 创建请求构建器
                let mut request_builder = client.request(method, url);

                // 处理请求头
                let processed_headers = self.process_headers(headers, upstream_config)?;
                request_builder = request_builder.headers(processed_headers);

                // 添加认证信息
                if let Some(ref auth) = upstream_config.auth {
                    request_builder = add_auth(request_builder, auth)?;
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

        // 执行请求
        let response = self
            .execute_request(
                &managed_upstream,
                upstream_url.as_str(),
                request_future(headers, body),
                group_name,
            )
            .await;

        // 记录上游请求耗时
        let duration = start_time.elapsed();
        METRICS
            .upstream_duration_seconds()
            .with_label_values(&[group_name, upstream_url.as_str()])
            .observe(duration.as_secs_f64());

        // 获取上游组的负载均衡器
        let load_balancer = self.groups.get(group_name).unwrap();

        // 更新响应时间感知的负载均衡器指标
        self.update_balancer_metrics(load_balancer, &managed_upstream, duration);

        // 错误处理和指标记录
        if let Err(ref err) = response {
            warn!(
                "Upstream request failed, reporting failure. Group: '{}', Upstream: '{}', Error: {}",
                group_name, &managed_upstream.upstream_ref.name, err
            );

            // 报告上游失败
            load_balancer.report_failure(&managed_upstream).await;

            // 记录错误指标
            let error_label = match err {
                AppError::CircuitBreakerOpen(_) => "circuit_open",
                AppError::Upstream(_) => error_labels::UPSTREAM_ERROR,
                _ => error_labels::UNKNOWN_ERROR,
            };

            METRICS
                .upstream_errors_total()
                .with_label_values(&[error_label, group_name, &managed_upstream.upstream_ref.name])
                .inc();
        } else if let Ok(ref response) = response {
            // 记录响应状态码
            let status = response.status().as_u16();
            debug!(
                "Upstream response status: {} from {}",
                status,
                upstream_url.as_str()
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
        // 如果没有头部操作需要执行，直接返回原始headers
        if upstream.headers.is_empty() {
            return Ok(headers);
        }

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
}
