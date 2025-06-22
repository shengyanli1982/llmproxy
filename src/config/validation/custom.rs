// This file will contain custom validation functions.
use validator::ValidationError;

use crate::config::{
    http_client::HttpClientConfig, upstream::AuthConfig, upstream::AuthType, upstream::HeaderOp,
    upstream::HeaderOpType, upstream_group::BalanceStrategy, upstream_group::UpstreamGroupConfig,
    Config, ProxyConfig,
};
use crate::r#const::http_client_limits;
use std::collections::HashSet;

pub fn validate_proxy_config(proxy: &ProxyConfig) -> Result<(), ValidationError> {
    if proxy.url.is_empty() {
        let mut err = ValidationError::new("url_empty");
        err.message = Some("Proxy URL cannot be empty".into());
        return Err(err);
    }
    if url::Url::parse(&proxy.url).is_err() {
        let mut err = ValidationError::new("url_invalid");
        err.message = Some("Proxy URL is not a valid URL".into());
        return Err(err);
    }
    Ok(())
}

pub fn validate_http_client_config(config: &HttpClientConfig) -> Result<(), ValidationError> {
    // 在流式模式下不检查请求超时，因为根据配置文件说明，流式模式下request_timeout被禁用
    if !config.stream_mode && config.timeout.request < http_client_limits::DEFAULT_REQUEST_TIMEOUT {
        let mut err = ValidationError::new("request_timeout_too_short");
        err.message = Some(
            format!(
                "Request timeout is too short, recommended minimum is {}s",
                http_client_limits::DEFAULT_REQUEST_TIMEOUT
            )
            .into(),
        );
        return Err(err);
    }

    Ok(())
}

pub fn validate_auth_config(auth: &AuthConfig) -> Result<(), ValidationError> {
    match auth.r#type {
        AuthType::Bearer => {
            if auth.token.as_ref().is_none_or(|s| s.is_empty()) {
                let mut err = ValidationError::new("bearer_token_empty");
                err.message = Some("Bearer token cannot be empty".into());
                return Err(err);
            }
        }
        AuthType::Basic => {
            if auth.username.as_ref().is_none_or(|s| s.is_empty())
                || auth.password.as_ref().is_none_or(|s| s.is_empty())
            {
                let mut err = ValidationError::new("basic_credentials_empty");
                err.message = Some("Basic auth requires a non-empty username and password".into());
                return Err(err);
            }
        }
        AuthType::None => {}
    }
    Ok(())
}

pub fn validate_header_op(op: &HeaderOp) -> Result<(), ValidationError> {
    match op.op {
        HeaderOpType::Insert | HeaderOpType::Replace => {
            if op.value.as_ref().is_none_or(|s| s.is_empty()) {
                let mut err = ValidationError::new("header_value_empty");
                err.message =
                    Some("Header value cannot be empty for insert/replace operations".into());
                return Err(err);
            }
        }
        HeaderOpType::Remove => {}
    }
    Ok(())
}

pub fn validate_weighted_round_robin(group: &UpstreamGroupConfig) -> Result<(), ValidationError> {
    if group.balance.strategy == BalanceStrategy::WeightedRoundRobin
        && group.upstreams.iter().any(|u| u.weight == 0)
    {
        let mut err = ValidationError::new("zero_weight_in_weighted_group");
        err.message = Some(
            "All upstreams in a weighted round-robin group must have a weight greater than 0"
                .into(),
        );
        return Err(err);
    }
    Ok(())
}

pub fn validate_config(config: &Config) -> Result<(), ValidationError> {
    let mut upstream_names = HashSet::new();
    for upstream in &config.upstreams {
        if !upstream_names.insert(&upstream.name) {
            let mut err = ValidationError::new("duplicate_upstream_name");
            err.message = Some(format!("Duplicate upstream name found: {}", upstream.name).into());
            return Err(err);
        }
    }

    let mut group_names = HashSet::new();
    for group in &config.upstream_groups {
        if !group_names.insert(&group.name) {
            let mut err = ValidationError::new("duplicate_upstream_group_name");
            err.message =
                Some(format!("Duplicate upstream group name found: {}", group.name).into());
            return Err(err);
        }
        for upstream_ref in &group.upstreams {
            if !upstream_names.contains(&upstream_ref.name) {
                let mut err = ValidationError::new("unknown_upstream_reference");
                err.message = Some(
                    format!(
                        "Upstream group '{}' references an unknown upstream: {}",
                        group.name, upstream_ref.name
                    )
                    .into(),
                );
                return Err(err);
            }
        }
    }

    let mut forward_names = HashSet::new();
    if let Some(http_server) = config.http_server.as_ref() {
        for forward in &http_server.forwards {
            if !forward_names.insert(&forward.name) {
                let mut err = ValidationError::new("duplicate_forward_name");
                err.message =
                    Some(format!("Duplicate forward name found: {}", forward.name).into());
                return Err(err);
            }
            if !group_names.contains(&forward.default_group) {
                let mut err = ValidationError::new("unknown_upstream_group_reference");
                err.message = Some(
                    format!(
                        "Forward '{}' references an unknown upstream group: {}",
                        forward.name, forward.default_group
                    )
                    .into(),
                );
                return Err(err);
            }
        }
    }

    Ok(())
}
