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
            "上游配置验证失败",
            vec![ErrorDetail {
                resource: Some("Upstream".to_string()),
                field: Some("name".to_string()),
                issue: "名称不能为空".to_string(),
            }],
        ));
    }

    // 验证URL格式
    if let Err(e) = Url::parse(&upstream.url) {
        return Err(ApiError::validation_error_with_details(
            "上游配置验证失败",
            vec![ErrorDetail {
                resource: Some("Upstream".to_string()),
                field: Some("url".to_string()),
                issue: format!("URL格式无效: {}", e),
            }],
        ));
    }

    // 验证认证配置
    if let Some(auth) = &upstream.auth {
        match auth.r#type {
            AuthType::Bearer => {
                if auth.token.as_ref().map_or(true, |s| s.is_empty()) {
                    return Err(ApiError::validation_error_with_details(
                        "上游配置验证失败",
                        vec![ErrorDetail {
                            resource: Some("Upstream".to_string()),
                            field: Some("auth.token".to_string()),
                            issue: "使用Bearer认证时必须提供有效的token".to_string(),
                        }],
                    ));
                }
            }
            AuthType::Basic => {
                if auth.username.as_ref().map_or(true, |s| s.is_empty())
                    || auth.password.as_ref().map_or(true, |s| s.is_empty())
                {
                    return Err(ApiError::validation_error_with_details(
                        "上游配置验证失败",
                        vec![ErrorDetail {
                            resource: Some("Upstream".to_string()),
                            field: Some("auth.username/auth.password".to_string()),
                            issue: "使用Basic认证时必须提供有效的用户名和密码".to_string(),
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
                        "上游配置验证失败",
                        vec![ErrorDetail {
                            resource: Some("Upstream".to_string()),
                            field: Some(format!("headers[{}].value", header_op.key)),
                            issue: format!("{}操作需要提供有效的值", header_op.op),
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
                "上游配置验证失败",
                vec![ErrorDetail {
                    resource: Some("Upstream".to_string()),
                    field: Some("breaker.threshold".to_string()),
                    issue: format!(
                        "熔断器阈值必须在 {} 和 {} 之间",
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
                "上游配置验证失败",
                vec![ErrorDetail {
                    resource: Some("Upstream".to_string()),
                    field: Some("breaker.cooldown".to_string()),
                    issue: format!(
                        "熔断器冷却时间必须在 {} 和 {} 之间",
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
            "上游组配置验证失败",
            vec![ErrorDetail {
                resource: Some("UpstreamGroup".to_string()),
                field: Some("name".to_string()),
                issue: "名称不能为空".to_string(),
            }],
        ));
    }

    // 检查上游列表
    if group.upstreams.is_empty() {
        return Err(ApiError::validation_error_with_details(
            "上游组配置验证失败",
            vec![ErrorDetail {
                resource: Some("UpstreamGroup".to_string()),
                field: Some("upstreams".to_string()),
                issue: "上游组必须至少包含一个上游".to_string(),
            }],
        ));
    }

    // 检查权重
    for (i, upstream_ref) in group.upstreams.iter().enumerate() {
        if upstream_ref.weight < weight_limits::MIN_WEIGHT
            || upstream_ref.weight > weight_limits::MAX_WEIGHT
        {
            return Err(ApiError::validation_error_with_details(
                "上游组配置验证失败",
                vec![ErrorDetail {
                    resource: Some("UpstreamGroup".to_string()),
                    field: Some(format!("upstreams[{}].weight", i)),
                    issue: format!(
                        "权重值必须在 {} 和 {} 之间",
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
                "上游组配置验证失败",
                vec![ErrorDetail {
                    resource: Some("UpstreamGroup".to_string()),
                    field: Some("balance.strategy".to_string()),
                    issue: "使用加权轮询策略时，至少一个上游的权重必须大于默认值".to_string(),
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
            "上游组配置验证失败",
            vec![ErrorDetail {
                resource: Some("UpstreamGroup".to_string()),
                field: Some("http_client.timeout.connect".to_string()),
                issue: format!(
                    "连接超时必须在 {} 和 {} 之间",
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
            "上游组配置验证失败",
            vec![ErrorDetail {
                resource: Some("UpstreamGroup".to_string()),
                field: Some("http_client.timeout.request".to_string()),
                issue: format!(
                    "请求超时必须在 {} 和 {} 之间",
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
            "上游组配置验证失败",
            vec![ErrorDetail {
                resource: Some("UpstreamGroup".to_string()),
                field: Some("http_client.timeout.idle".to_string()),
                issue: format!(
                    "空闲超时必须在 {} 和 {} 之间",
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
                "上游组配置验证失败",
                vec![ErrorDetail {
                    resource: Some("UpstreamGroup".to_string()),
                    field: Some("http_client.retry.attempts".to_string()),
                    issue: format!(
                        "重试次数必须在 {} 和 {} 之间",
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
                "上游组配置验证失败",
                vec![ErrorDetail {
                    resource: Some("UpstreamGroup".to_string()),
                    field: Some("http_client.retry.initial".to_string()),
                    issue: format!(
                        "初始重试间隔必须在 {} 和 {} 之间",
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
            "转发服务配置验证失败",
            vec![ErrorDetail {
                resource: Some("Forward".to_string()),
                field: Some("name".to_string()),
                issue: "名称不能为空".to_string(),
            }],
        ));
    }

    // 检查上游组
    if forward.upstream_group.is_empty() {
        return Err(ApiError::validation_error_with_details(
            "转发服务配置验证失败",
            vec![ErrorDetail {
                resource: Some("Forward".to_string()),
                field: Some("upstream_group".to_string()),
                issue: "上游组名称不能为空".to_string(),
            }],
        ));
    }

    // 验证限流配置
    if forward.ratelimit.enabled {
        if forward.ratelimit.per_second < rate_limit_limits::MIN_RATE
            || forward.ratelimit.per_second > rate_limit_limits::MAX_RATE
        {
            return Err(ApiError::validation_error_with_details(
                "转发服务配置验证失败",
                vec![ErrorDetail {
                    resource: Some("Forward".to_string()),
                    field: Some("ratelimit.per_second".to_string()),
                    issue: format!(
                        "每秒请求数必须在 {} 和 {} 之间",
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
                "转发服务配置验证失败",
                vec![ErrorDetail {
                    resource: Some("Forward".to_string()),
                    field: Some("ratelimit.burst".to_string()),
                    issue: format!(
                        "突发请求数必须大于等于每秒请求数，且不超过 {}",
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
            "转发服务配置验证失败",
            vec![ErrorDetail {
                resource: Some("Forward".to_string()),
                field: Some("timeout.connect".to_string()),
                issue: format!(
                    "连接超时必须在 {} 和 {} 之间",
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
            debug!("配置验证失败: {}", msg);
            Err(ApiError::validation_error(msg))
        }
        Err(e) => {
            debug!("配置验证失败: {}", e);
            Err(ApiError::internal_server_error(format!(
                "配置验证失败: {}",
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
                    "无法删除上游 '{}': 它正被上游组 '{}' 引用",
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
                "无法删除上游组 '{}': 它正被转发服务 '{}' 引用",
                group_name, forward.name
            )));
        }
    }
    Ok(())
}
