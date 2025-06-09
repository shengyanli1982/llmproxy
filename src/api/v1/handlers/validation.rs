use crate::api::v1::models::{ApiError, ErrorDetail};
use crate::config::upstream::{AuthType, HeaderOpType};
use crate::config::upstream_group::BalanceStrategy;
use crate::config::{Config, ForwardConfig, UpstreamConfig, UpstreamGroupConfig};
use crate::error::AppError;
use crate::r#const::{
    breaker_limits, http_client_limits, rate_limit_limits, retry_limits, weight_limits,
};
use tracing::debug;
use url::Url;

/// 验证上游配置载荷
pub fn validate_upstream_payload(upstream: &UpstreamConfig) -> Result<(), ApiError> {
    // 检查名称
    if upstream.name.is_empty() {
        return Err(ApiError::validation_error_with_details(
            "Upstream configuration validation failed",
            vec![ErrorDetail {
                resource: Some("Upstream".to_string()),
                field: Some("name".to_string()),
                issue: "Name cannot be empty".to_string(),
            }],
        ));
    }

    // 验证URL格式
    if let Err(e) = Url::parse(&upstream.url) {
        return Err(ApiError::validation_error_with_details(
            "Upstream configuration validation failed",
            vec![ErrorDetail {
                resource: Some("Upstream".to_string()),
                field: Some("url".to_string()),
                issue: format!("Invalid URL format: {}", e),
            }],
        ));
    }

    // 验证认证配置
    if let Some(auth) = &upstream.auth {
        match auth.r#type {
            AuthType::Bearer => {
                if auth.token.as_ref().map_or(true, |s| s.is_empty()) {
                    return Err(ApiError::validation_error_with_details(
                        "Upstream configuration validation failed",
                        vec![ErrorDetail {
                            resource: Some("Upstream".to_string()),
                            field: Some("auth.token".to_string()),
                            issue: "Valid token must be provided when using Bearer authentication"
                                .to_string(),
                        }],
                    ));
                }
            }
            AuthType::Basic => {
                if auth.username.as_ref().map_or(true, |s| s.is_empty())
                    || auth.password.as_ref().map_or(true, |s| s.is_empty())
                {
                    return Err(ApiError::validation_error_with_details(
                        "Upstream configuration validation failed",
                        vec![ErrorDetail {
                            resource: Some("Upstream".to_string()),
                            field: Some("auth.username/auth.password".to_string()),
                            issue: "Valid username and password must be provided when using Basic authentication".to_string(),
                        }],
                    ));
                }
            }
            AuthType::None => {}
        }
    }

    // 验证请求头操作
    for header_op in &upstream.headers {
        match header_op.op {
            HeaderOpType::Insert | HeaderOpType::Replace => {
                if header_op.value.as_ref().map_or(true, |s| s.is_empty()) {
                    return Err(ApiError::validation_error_with_details(
                        "Upstream configuration validation failed",
                        vec![ErrorDetail {
                            resource: Some("Upstream".to_string()),
                            field: Some(format!("headers[{}].value", header_op.key)),
                            issue: format!("{} operation requires a valid value", header_op.op),
                        }],
                    ));
                }
            }
            HeaderOpType::Remove => {}
        }
    }

    // 验证熔断器配置
    if let Some(breaker) = &upstream.breaker {
        if breaker.threshold < breaker_limits::MIN_THRESHOLD
            || breaker.threshold > breaker_limits::MAX_THRESHOLD
        {
            return Err(ApiError::validation_error_with_details(
                "Upstream configuration validation failed",
                vec![ErrorDetail {
                    resource: Some("Upstream".to_string()),
                    field: Some("breaker.threshold".to_string()),
                    issue: format!(
                        "Circuit breaker threshold must be between {} and {}",
                        breaker_limits::MIN_THRESHOLD,
                        breaker_limits::MAX_THRESHOLD
                    ),
                }],
            ));
        }

        if breaker.cooldown < breaker_limits::MIN_COOLDOWN
            || breaker.cooldown > breaker_limits::MAX_COOLDOWN
        {
            return Err(ApiError::validation_error_with_details(
                "Upstream configuration validation failed",
                vec![ErrorDetail {
                    resource: Some("Upstream".to_string()),
                    field: Some("breaker.cooldown".to_string()),
                    issue: format!(
                        "Circuit breaker cooldown must be between {} and {}",
                        breaker_limits::MIN_COOLDOWN,
                        breaker_limits::MAX_COOLDOWN
                    ),
                }],
            ));
        }
    }

    Ok(())
}

/// 验证上游组配置载荷
pub fn validate_upstream_group_payload(group: &UpstreamGroupConfig) -> Result<(), ApiError> {
    // 检查名称
    if group.name.is_empty() {
        return Err(ApiError::validation_error_with_details(
            "Upstream group configuration validation failed",
            vec![ErrorDetail {
                resource: Some("UpstreamGroup".to_string()),
                field: Some("name".to_string()),
                issue: "Name cannot be empty".to_string(),
            }],
        ));
    }

    // 检查上游列表
    if group.upstreams.is_empty() {
        return Err(ApiError::validation_error_with_details(
            "Upstream group configuration validation failed",
            vec![ErrorDetail {
                resource: Some("UpstreamGroup".to_string()),
                field: Some("upstreams".to_string()),
                issue: "Upstream group must contain at least one upstream".to_string(),
            }],
        ));
    }

    // 检查权重
    for (i, upstream_ref) in group.upstreams.iter().enumerate() {
        if upstream_ref.weight < weight_limits::MIN_WEIGHT
            || upstream_ref.weight > weight_limits::MAX_WEIGHT
        {
            return Err(ApiError::validation_error_with_details(
                "Upstream group configuration validation failed",
                vec![ErrorDetail {
                    resource: Some("UpstreamGroup".to_string()),
                    field: Some(format!("upstreams[{}].weight", i)),
                    issue: format!(
                        "Weight must be between {} and {}",
                        weight_limits::MIN_WEIGHT,
                        weight_limits::MAX_WEIGHT
                    ),
                }],
            ));
        }
    }

    // 验证负载均衡策略
    if group.balance.strategy == BalanceStrategy::WeightedRoundRobin {
        let all_default_weight = group
            .upstreams
            .iter()
            .all(|u| u.weight == weight_limits::MIN_WEIGHT);
        if all_default_weight {
            return Err(ApiError::validation_error_with_details(
                "Upstream group configuration validation failed",
                vec![ErrorDetail {
                    resource: Some("UpstreamGroup".to_string()),
                    field: Some("balance.strategy".to_string()),
                    issue: "When using weighted round-robin strategy, at least one upstream must have weight greater than the default value".to_string(),
                }],
            ));
        }
    }

    // 验证HTTP客户端配置
    let http_client = &group.http_client;

    // 验证超时配置
    if http_client.timeout.connect < http_client_limits::MIN_CONNECT_TIMEOUT
        || http_client.timeout.connect > http_client_limits::MAX_CONNECT_TIMEOUT
    {
        return Err(ApiError::validation_error_with_details(
            "Upstream group configuration validation failed",
            vec![ErrorDetail {
                resource: Some("UpstreamGroup".to_string()),
                field: Some("http_client.timeout.connect".to_string()),
                issue: format!(
                    "Connection timeout must be between {} and {}",
                    http_client_limits::MIN_CONNECT_TIMEOUT,
                    http_client_limits::MAX_CONNECT_TIMEOUT
                ),
            }],
        ));
    }

    if http_client.timeout.request < http_client_limits::MIN_REQUEST_TIMEOUT
        || http_client.timeout.request > http_client_limits::MAX_REQUEST_TIMEOUT
    {
        return Err(ApiError::validation_error_with_details(
            "Upstream group configuration validation failed",
            vec![ErrorDetail {
                resource: Some("UpstreamGroup".to_string()),
                field: Some("http_client.timeout.request".to_string()),
                issue: format!(
                    "Request timeout must be between {} and {}",
                    http_client_limits::MIN_REQUEST_TIMEOUT,
                    http_client_limits::MAX_REQUEST_TIMEOUT
                ),
            }],
        ));
    }

    if http_client.timeout.idle < http_client_limits::MIN_IDLE_TIMEOUT
        || http_client.timeout.idle > http_client_limits::MAX_IDLE_TIMEOUT
    {
        return Err(ApiError::validation_error_with_details(
            "Upstream group configuration validation failed",
            vec![ErrorDetail {
                resource: Some("UpstreamGroup".to_string()),
                field: Some("http_client.timeout.idle".to_string()),
                issue: format!(
                    "Idle timeout must be between {} and {}",
                    http_client_limits::MIN_IDLE_TIMEOUT,
                    http_client_limits::MAX_IDLE_TIMEOUT
                ),
            }],
        ));
    }

    // 验证重试配置
    if http_client.retry.enabled {
        if http_client.retry.attempts < retry_limits::MIN_ATTEMPTS
            || http_client.retry.attempts > retry_limits::MAX_ATTEMPTS
        {
            return Err(ApiError::validation_error_with_details(
                "Upstream group configuration validation failed",
                vec![ErrorDetail {
                    resource: Some("UpstreamGroup".to_string()),
                    field: Some("http_client.retry.attempts".to_string()),
                    issue: format!(
                        "Retry attempts must be between {} and {}",
                        retry_limits::MIN_ATTEMPTS,
                        retry_limits::MAX_ATTEMPTS
                    ),
                }],
            ));
        }

        if http_client.retry.initial < retry_limits::MIN_INITIAL_BACKOFF
            || http_client.retry.initial > retry_limits::MAX_INITIAL_BACKOFF
        {
            return Err(ApiError::validation_error_with_details(
                "Upstream group configuration validation failed",
                vec![ErrorDetail {
                    resource: Some("UpstreamGroup".to_string()),
                    field: Some("http_client.retry.initial".to_string()),
                    issue: format!(
                        "Initial retry interval must be between {} and {}",
                        retry_limits::MIN_INITIAL_BACKOFF,
                        retry_limits::MAX_INITIAL_BACKOFF
                    ),
                }],
            ));
        }
    }

    Ok(())
}

/// 验证转发服务配置载荷
pub fn validate_forward_payload(forward: &ForwardConfig) -> Result<(), ApiError> {
    // 检查名称
    if forward.name.is_empty() {
        return Err(ApiError::validation_error_with_details(
            "Forward service configuration validation failed",
            vec![ErrorDetail {
                resource: Some("Forward".to_string()),
                field: Some("name".to_string()),
                issue: "Name cannot be empty".to_string(),
            }],
        ));
    }

    // 检查上游组
    if forward.upstream_group.is_empty() {
        return Err(ApiError::validation_error_with_details(
            "Forward service configuration validation failed",
            vec![ErrorDetail {
                resource: Some("Forward".to_string()),
                field: Some("upstream_group".to_string()),
                issue: "Upstream group name cannot be empty".to_string(),
            }],
        ));
    }

    // 验证限流配置
    if forward.ratelimit.enabled {
        if forward.ratelimit.per_second < rate_limit_limits::MIN_RATE
            || forward.ratelimit.per_second > rate_limit_limits::MAX_RATE
        {
            return Err(ApiError::validation_error_with_details(
                "Forward service configuration validation failed",
                vec![ErrorDetail {
                    resource: Some("Forward".to_string()),
                    field: Some("ratelimit.per_second".to_string()),
                    issue: format!(
                        "Requests per second must be between {} and {}",
                        rate_limit_limits::MIN_RATE,
                        rate_limit_limits::MAX_RATE
                    ),
                }],
            ));
        }

        if forward.ratelimit.burst < forward.ratelimit.per_second
            || forward.ratelimit.burst > rate_limit_limits::MAX_BURST
        {
            return Err(ApiError::validation_error_with_details(
                "Forward service configuration validation failed",
                vec![ErrorDetail {
                    resource: Some("Forward".to_string()),
                    field: Some("ratelimit.burst".to_string()),
                    issue: format!(
                        "Burst size must be greater than or equal to requests per second and not exceed {}",
                        rate_limit_limits::MAX_BURST
                    ),
                }],
            ));
        }
    }

    // 验证超时配置
    if forward.timeout.connect < http_client_limits::MIN_CONNECT_TIMEOUT
        || forward.timeout.connect > http_client_limits::MAX_CONNECT_TIMEOUT
    {
        return Err(ApiError::validation_error_with_details(
            "Forward service configuration validation failed",
            vec![ErrorDetail {
                resource: Some("Forward".to_string()),
                field: Some("timeout.connect".to_string()),
                issue: format!(
                    "Connection timeout must be between {} and {}",
                    http_client_limits::MIN_CONNECT_TIMEOUT,
                    http_client_limits::MAX_CONNECT_TIMEOUT
                ),
            }],
        ));
    }

    Ok(())
}

/// 检查配置的引用完整性
pub fn check_config_integrity(config: &Config) -> Result<(), ApiError> {
    // 转换AppError到ApiError
    match config.validate() {
        Ok(_) => Ok(()),
        Err(AppError::Config(msg)) => {
            debug!("Configuration validation failed: {}", msg);
            Err(ApiError::validation_error(msg))
        }
        Err(e) => {
            debug!("Configuration validation failed: {}", e);
            Err(ApiError::internal_server_error(format!(
                "Configuration validation failed: {}",
                e
            )))
        }
    }
}

/// 检查上游是否被任何上游组引用
pub fn check_upstream_references(config: &Config, upstream_name: &str) -> Result<(), ApiError> {
    for group in &config.upstream_groups {
        for upstream_ref in &group.upstreams {
            if upstream_ref.name == upstream_name {
                return Err(ApiError::resource_conflict(format!(
                    "Cannot delete upstream '{}': it is referenced by upstream group '{}'",
                    upstream_name, group.name
                )));
            }
        }
    }
    Ok(())
}

/// 检查上游组是否被任何转发服务引用
pub fn check_upstream_group_references(config: &Config, group_name: &str) -> Result<(), ApiError> {
    for forward in &config.http_server.forwards {
        if forward.upstream_group == group_name {
            return Err(ApiError::resource_conflict(format!(
                "Cannot delete upstream group '{}': it is referenced by forward service '{}'",
                group_name, forward.name
            )));
        }
    }
    Ok(())
}
