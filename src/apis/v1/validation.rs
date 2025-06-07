use crate::apis::v1::error::ApiError;
use crate::config::{
    AuthType, BalanceStrategy, Config, ForwardConfig, HeaderOpType, UpstreamConfig,
    UpstreamGroupConfig,
};
use crate::r#const::{
    breaker_limits, http_client_limits, rate_limit_limits, retry_limits, weight_limits,
};
use std::collections::HashSet;
use url::Url;

/// 为 Config 添加校验辅助方法
pub trait ConfigValidation {
    fn validate_upstream_config(&self, upstream: &UpstreamConfig) -> Result<(), ApiError>;
    fn validate_upstream_group_config(&self, group: &UpstreamGroupConfig) -> Result<(), ApiError>;
    fn validate_forward_config(&self, forward: &ForwardConfig) -> Result<(), ApiError>;
}

impl ConfigValidation for Config {
    fn validate_upstream_config(&self, upstream: &UpstreamConfig) -> Result<(), ApiError> {
        // 验证URL
        if let Err(e) = Url::parse(&upstream.url) {
            return Err(ApiError::ValidationError(format!("invalid URL: {}", e)));
        }

        // 确保 URL 包含协议和主机
        let url = Url::parse(&upstream.url).unwrap(); // 前面已经检查过解析错误
        if url.scheme().is_empty() || url.host_str().is_none() {
            return Err(ApiError::ValidationError(
                "invalid URL: missing scheme or host".into(),
            ));
        }

        // 验证认证配置
        if let Some(auth) = &upstream.auth {
            match auth.r#type {
                AuthType::Bearer => {
                    if auth.token.as_ref().is_none_or(|s| s.is_empty()) {
                        return Err(ApiError::ValidationError(
                            "Bearer authentication requires a non-empty token".into(),
                        ));
                    }
                }
                AuthType::Basic => {
                    if auth.username.as_ref().is_none_or(|s| s.is_empty()) {
                        return Err(ApiError::ValidationError(
                            "Basic authentication requires a non-empty username".into(),
                        ));
                    }
                    if auth.password.as_ref().is_none_or(|s| s.is_empty()) {
                        return Err(ApiError::ValidationError(
                            "Basic authentication requires a non-empty password".into(),
                        ));
                    }
                }
                _ => {}
            }
        }

        // 验证请求头操作
        for header in &upstream.headers {
            match header.op {
                HeaderOpType::Insert | HeaderOpType::Replace => {
                    if header.value.as_ref().is_none_or(|s| s.is_empty()) {
                        return Err(ApiError::ValidationError(format!(
                            "Header operation {:?} requires a non-empty value",
                            header.op
                        )));
                    }
                }
                _ => {}
            }
        }

        // 验证熔断器配置
        if let Some(breaker) = &upstream.breaker {
            if breaker.threshold < breaker_limits::MIN_THRESHOLD
                || breaker.threshold > breaker_limits::MAX_THRESHOLD
            {
                return Err(ApiError::ValidationError(format!(
                    "Breaker threshold must be between {} and {}",
                    breaker_limits::MIN_THRESHOLD,
                    breaker_limits::MAX_THRESHOLD
                )));
            }

            if breaker.cooldown < breaker_limits::MIN_COOLDOWN
                || breaker.cooldown > breaker_limits::MAX_COOLDOWN
            {
                return Err(ApiError::ValidationError(format!(
                    "Breaker cooldown must be between {} and {} seconds",
                    breaker_limits::MIN_COOLDOWN,
                    breaker_limits::MAX_COOLDOWN
                )));
            }
        }

        Ok(())
    }

    fn validate_upstream_group_config(&self, group: &UpstreamGroupConfig) -> Result<(), ApiError> {
        // 验证上游引用列表
        if group.upstreams.is_empty() {
            return Err(ApiError::ValidationError(
                "Upstream group must have at least one upstream, empty list is not allowed".into(),
            ));
        }

        // 验证引用的上游是否存在
        let upstream_names: HashSet<_> = self.upstreams.iter().map(|u| u.name.clone()).collect();

        for upstream_ref in &group.upstreams {
            if !upstream_names.contains(&upstream_ref.name) {
                return Err(ApiError::ReferenceNotFound {
                    resource_type: "Upstream".into(),
                    name: upstream_ref.name.clone(),
                });
            }
        }

        // 验证权重
        for upstream_ref in &group.upstreams {
            if upstream_ref.weight < weight_limits::MIN_WEIGHT
                || upstream_ref.weight > weight_limits::MAX_WEIGHT
            {
                return Err(ApiError::ValidationError(format!(
                    "Weight for upstream '{}' must be between {} and {}",
                    upstream_ref.name,
                    weight_limits::MIN_WEIGHT,
                    weight_limits::MAX_WEIGHT
                )));
            }
        }

        // 验证负载均衡策略
        if group.balance.strategy == BalanceStrategy::WeightedRoundRobin {
            // 检查是否所有上游都有相同的权重
            let all_default_weight = group
                .upstreams
                .iter()
                .all(|u| u.weight == weight_limits::MIN_WEIGHT);

            if all_default_weight && group.upstreams.len() > 1 {
                return Err(ApiError::ValidationError(
                    "When using weighted_roundrobin strategy, not all upstreams should have the default weight".into()
                ));
            }
        }

        // 验证HTTP客户端配置

        // 验证超时配置
        if group.http_client.timeout.connect < http_client_limits::MIN_CONNECT_TIMEOUT
            || group.http_client.timeout.connect > http_client_limits::MAX_CONNECT_TIMEOUT
        {
            return Err(ApiError::ValidationError(format!(
                "Connect timeout must be between {} and {} seconds",
                http_client_limits::MIN_CONNECT_TIMEOUT,
                http_client_limits::MAX_CONNECT_TIMEOUT
            )));
        }

        if group.http_client.timeout.request < http_client_limits::MIN_REQUEST_TIMEOUT
            || group.http_client.timeout.request > http_client_limits::MAX_REQUEST_TIMEOUT
        {
            return Err(ApiError::ValidationError(format!(
                "Request timeout must be between {} and {} seconds",
                http_client_limits::MIN_REQUEST_TIMEOUT,
                http_client_limits::MAX_REQUEST_TIMEOUT
            )));
        }

        if group.http_client.timeout.idle < http_client_limits::MIN_IDLE_TIMEOUT
            || group.http_client.timeout.idle > http_client_limits::MAX_IDLE_TIMEOUT
        {
            return Err(ApiError::ValidationError(format!(
                "Idle timeout must be between {} and {} seconds",
                http_client_limits::MIN_IDLE_TIMEOUT,
                http_client_limits::MAX_IDLE_TIMEOUT
            )));
        }

        // 验证keepalive
        if group.http_client.keepalive < http_client_limits::MIN_KEEPALIVE
            || group.http_client.keepalive > http_client_limits::MAX_KEEPALIVE
        {
            return Err(ApiError::ValidationError(format!(
                "Keepalive must be between {} and {} seconds",
                http_client_limits::MIN_KEEPALIVE,
                http_client_limits::MAX_KEEPALIVE
            )));
        }

        // 验证重试配置
        if group.http_client.retry.enabled {
            if group.http_client.retry.attempts < retry_limits::MIN_ATTEMPTS
                || group.http_client.retry.attempts > retry_limits::MAX_ATTEMPTS
            {
                return Err(ApiError::ValidationError(format!(
                    "Retry attempts must be between {} and {}",
                    retry_limits::MIN_ATTEMPTS,
                    retry_limits::MAX_ATTEMPTS
                )));
            }

            if group.http_client.retry.initial < retry_limits::MIN_INITIAL_MS
                || group.http_client.retry.initial > retry_limits::MAX_INITIAL_MS
            {
                return Err(ApiError::ValidationError(format!(
                    "Retry initial delay must be between {} and {} milliseconds",
                    retry_limits::MIN_INITIAL_MS,
                    retry_limits::MAX_INITIAL_MS
                )));
            }
        }

        // 验证代理配置
        if group.http_client.proxy.enabled && group.http_client.proxy.url.is_empty() {
            return Err(ApiError::ValidationError(
                "Proxy URL cannot be empty when proxy is enabled".into(),
            ));
        }

        Ok(())
    }

    fn validate_forward_config(&self, forward: &ForwardConfig) -> Result<(), ApiError> {
        // 验证端口
        if forward.port == 0 {
            return Err(ApiError::ValidationError(
                "Invalid port: cannot be 0".into(),
            ));
        }

        // 验证地址
        if forward.address.is_empty() {
            return Err(ApiError::ValidationError(
                "Invalid address: cannot be empty".into(),
            ));
        }

        // 验证引用的上游组是否存在
        let group_exists = self
            .upstream_groups
            .iter()
            .any(|g| g.name == forward.upstream_group);
        if !group_exists {
            return Err(ApiError::ReferenceNotFound {
                resource_type: "UpstreamGroup".into(),
                name: forward.upstream_group.clone(),
            });
        }

        // 验证限流配置
        if forward.ratelimit.enabled {
            if forward.ratelimit.per_second < rate_limit_limits::MIN_PER_SECOND
                || forward.ratelimit.per_second > rate_limit_limits::MAX_PER_SECOND
            {
                return Err(ApiError::ValidationError(format!(
                    "Rate limit per_second must be between {} and {}",
                    rate_limit_limits::MIN_PER_SECOND,
                    rate_limit_limits::MAX_PER_SECOND
                )));
            }

            if forward.ratelimit.burst < rate_limit_limits::MIN_BURST
                || forward.ratelimit.burst > rate_limit_limits::MAX_BURST
            {
                return Err(ApiError::ValidationError(format!(
                    "Rate limit burst must be between {} and {}",
                    rate_limit_limits::MIN_BURST,
                    rate_limit_limits::MAX_BURST
                )));
            }
        } else {
            // 即使未启用限流，也验证配置值的有效性
            if forward.ratelimit.per_second == 0 {
                return Err(ApiError::ValidationError(
                    "Rate limit per_second cannot be 0, even when rate limiting is disabled".into(),
                ));
            }
        }

        // 验证超时配置
        if forward.timeout.connect < http_client_limits::MIN_CONNECT_TIMEOUT
            || forward.timeout.connect > http_client_limits::MAX_CONNECT_TIMEOUT
        {
            return Err(ApiError::ValidationError(format!(
                "Connect timeout must be between {} and {} seconds",
                http_client_limits::MIN_CONNECT_TIMEOUT,
                http_client_limits::MAX_CONNECT_TIMEOUT
            )));
        }

        Ok(())
    }
}
