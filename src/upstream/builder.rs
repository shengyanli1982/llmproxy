use crate::{
    balancer::ManagedUpstream,
    breaker::create_upstream_circuit_breaker,
    config::{UpstreamConfig, UpstreamRef},
    error::AppError,
};
use std::collections::HashMap;
use tracing::debug;

/// 构建上游配置映射
pub(super) fn build_upstream_map(upstreams: &[UpstreamConfig]) -> HashMap<String, UpstreamConfig> {
    let mut upstream_map = HashMap::with_capacity(upstreams.len());

    for upstream in upstreams {
        debug!(
            "Loaded upstream: {:?}, url: {:?}",
            upstream.name, upstream.url
        );
        upstream_map.insert(upstream.name.clone(), upstream.clone());
    }

    upstream_map
}

/// 创建托管上游
pub(super) fn create_managed_upstream(
    upstream_ref: &UpstreamRef,
    upstream_config: &UpstreamConfig,
    group_name: &str,
) -> Result<ManagedUpstream, AppError> {
    // 创建熔断器（如果上游配置了熔断器）
    let breaker = match &upstream_config.breaker {
        Some(breaker_config) => {
            let breaker = create_upstream_circuit_breaker(
                upstream_ref.name.clone(),
                group_name.to_string(),
                breaker_config,
            );
            Some(breaker)
        }
        None => None,
    };

    // 创建托管上游
    let managed_upstream = ManagedUpstream {
        upstream_ref: upstream_ref.clone(),
        breaker,
    };

    Ok(managed_upstream)
}
