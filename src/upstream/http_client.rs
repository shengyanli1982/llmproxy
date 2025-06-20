use crate::{
    config::{AuthConfig, AuthType, HttpClientConfig, UpstreamGroupConfig},
    error::AppError,
    r#const::retry_limits,
};
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use retry_policies::Jitter;
use std::{collections::HashMap, time::Duration};
use tracing::debug;

/// 为多个上游组创建HTTP客户端映射
pub(super) fn create_group_clients(
    groups: &[UpstreamGroupConfig],
) -> Result<HashMap<String, ClientWithMiddleware>, AppError> {
    let mut group_clients = HashMap::with_capacity(groups.len());

    for group in groups {
        // 创建该组的HTTP客户端
        let client = create_http_client(&group.http_client)?;
        group_clients.insert(group.name.clone(), client);
    }

    Ok(group_clients)
}

/// 创建HTTP客户端
pub(super) fn create_http_client(
    config: &HttpClientConfig,
) -> Result<ClientWithMiddleware, AppError> {
    debug!("Creating HTTP client, config: {:?}", config);

    // 创建 reqwest 客户端
    let mut client_builder = reqwest::Client::builder()
        .tcp_keepalive(Some(Duration::from_secs(config.keepalive.into())))
        .connect_timeout(Duration::from_secs(config.timeout.connect));

    // 如果未启用流式模式，则设置请求超时
    if !config.stream_mode {
        client_builder = client_builder.timeout(Duration::from_secs(config.timeout.request));
    }

    // 设置空闲超时
    if config.timeout.idle > 0 {
        client_builder =
            client_builder.pool_idle_timeout(Some(Duration::from_secs(config.timeout.idle)));
    }

    // 配置代理（如果启用）
    if let Some(proxy_config) = &config.proxy {
        if let Ok(proxy) = reqwest::Proxy::all(&proxy_config.url) {
            client_builder = client_builder.proxy(proxy);
        }
    }

    // 创建基础HTTP客户端
    let client = client_builder.build()?;

    // 配置重试策略（根据组的重试配置）
    let middleware_client = if let Some(retry_config) = &config.retry {
        // 使用指数退避策略，基于组的重试配置
        let retry_policy = ExponentialBackoff::builder()
            .retry_bounds(
                Duration::from_millis(retry_config.initial.into()),
                Duration::from_secs(retry_limits::MAX_DELAY.into()),
            )
            .base(2)
            .jitter(Jitter::Bounded)
            .build_with_max_retries(retry_config.attempts);

        reqwest_middleware::ClientBuilder::new(client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build()
    } else {
        // 不进行重试
        reqwest_middleware::ClientBuilder::new(client).build()
    };

    Ok(middleware_client)
}

/// 添加认证信息到请求
pub(super) fn add_auth(
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
