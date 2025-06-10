use std::collections::HashSet;

use crate::config::common::{BreakerConfig, RateLimitConfig, TimeoutConfig};
use crate::config::http_client::HttpClientConfig;
use crate::config::upstream::HeaderOpType;
use crate::error::AppError;
use crate::r#const::{
    breaker_limits, http_client_limits, rate_limit_limits, retry_limits, weight_limits,
};
use tracing::debug;
use url::Url;

use super::Config;

impl Config {
    // 验证配置
    pub fn validate(&self) -> Result<(), AppError> {
        // 验证名称唯一性
        self.validate_name_uniqueness()?;

        // 验证上游组
        let upstream_names: HashSet<&str> =
            self.upstreams.iter().map(|u| u.name.as_str()).collect();

        // 检查上游组中引用的上游是否存在
        for group in &self.upstream_groups {
            // 确保每个组至少有一个上游
            if group.upstreams.is_empty() {
                return Err(AppError::Config(format!(
                    "Upstream group '{}' has no defined upstreams",
                    group.name
                )));
            }

            for upstream_ref in &group.upstreams {
                // 检查引用的上游是否存在
                if !upstream_names.contains(upstream_ref.name.as_str()) {
                    return Err(AppError::Config(format!(
                        "Upstream group '{}' references non-existent upstream '{}'",
                        group.name, upstream_ref.name
                    )));
                }

                // 验证权重值
                if upstream_ref.weight < weight_limits::MIN_WEIGHT
                    || upstream_ref.weight > weight_limits::MAX_WEIGHT
                {
                    return Err(AppError::Config(format!(
                        "Weight {} for upstream '{}' in group '{}' is out of valid range [{}-{}]",
                        upstream_ref.weight,
                        upstream_ref.name,
                        group.name,
                        weight_limits::MIN_WEIGHT,
                        weight_limits::MAX_WEIGHT
                    )));
                }
            }

            // 验证负载均衡策略
            // 检查策略值是否有效
            use crate::config::upstream_group::BalanceStrategy;
            match group.balance.strategy {
                BalanceStrategy::RoundRobin => {}
                BalanceStrategy::WeightedRoundRobin => {
                    // 检查是否所有上游都有合理的权重设置
                    let all_default_weight = group
                        .upstreams
                        .iter()
                        .all(|u| u.weight == weight_limits::MIN_WEIGHT);
                    if all_default_weight {
                        return Err(AppError::Config(format!(
                            "Upstream group '{}' uses weighted_roundrobin strategy but all upstreams have default weight",
                            group.name
                        )));
                    }
                }
                BalanceStrategy::Random => {}
                BalanceStrategy::ResponseAware => {}
                BalanceStrategy::Failover => {
                    // 故障转移策略不需要特殊验证，但需要至少有两个上游才有意义
                    if group.upstreams.len() < 2 {
                        debug!(
                            "Upstream group '{}' uses failover strategy but has only {} upstream(s), consider adding more for better failover capability",
                            group.name,
                            group.upstreams.len()
                        );
                    }
                }
            }

            // 记录使用的负载均衡策略，用于日志或指标
            debug!(
                "Upstream group '{}' uses balance strategy: {}",
                group.name,
                group.balance.strategy.as_str()
            );

            // 验证 HTTP 客户端配置
            self.validate_http_client_config(
                &group.http_client,
                &format!("Upstream group '{}'", group.name),
            )?;
        }

        // 检查转发服务引用的上游组是否存在
        let group_names: HashSet<&str> = self
            .upstream_groups
            .iter()
            .map(|g| g.name.as_str())
            .collect();

        for forward in &self.http_server.forwards {
            // 检查引用的上游组是否存在
            if !group_names.contains(forward.upstream_group.as_str()) {
                return Err(AppError::Config(format!(
                    "Forwarding service '{}' references non-existent upstream group '{}'",
                    forward.name, forward.upstream_group
                )));
            }

            // 验证限流配置
            if forward.ratelimit.enabled {
                self.validate_rate_limit_config(&forward.ratelimit, &forward.name)?;
            }

            // 验证超时配置
            self.validate_timeout_config(
                &forward.timeout,
                &format!("Forwarding service '{}'", forward.name),
            )?;
        }

        // 验证管理服务的超时配置
        self.validate_timeout_config(&self.http_server.admin.timeout, "Admin service")?;

        // 验证上游配置
        for upstream in &self.upstreams {
            // 验证 URL 格式
            if let Err(e) = Url::parse(&upstream.url) {
                return Err(AppError::Config(format!(
                    "URL '{}' for upstream '{}' is invalid: {}",
                    upstream.url, upstream.name, e
                )));
            }

            // 验证熔断器配置
            if let Some(breaker) = &upstream.breaker {
                self.validate_breaker_config(breaker, &upstream.name)?;
            }

            // 验证认证配置
            if let Some(auth) = &upstream.auth {
                use crate::config::upstream::AuthType;
                match auth.r#type {
                    AuthType::Bearer => {
                        if auth.token.as_ref().is_none_or(|s| s.is_empty()) {
                            return Err(AppError::Config(format!(
                                "Upstream '{}' uses Bearer authentication but no valid token was provided",
                                upstream.name
                            )));
                        }
                    }
                    AuthType::Basic => {
                        if auth.username.as_ref().is_none_or(|s| s.is_empty())
                            || auth.password.as_ref().is_none_or(|s| s.is_empty())
                        {
                            return Err(AppError::Config(format!(
                                "Upstream '{}' uses Basic authentication but no valid username and password were provided",
                                upstream.name
                            )));
                        }
                    }
                    AuthType::None => {}
                }
            }

            // 验证请求头操作
            for header_op in &upstream.headers {
                match header_op.op {
                    HeaderOpType::Insert | HeaderOpType::Replace => {
                        if header_op.value.as_ref().is_none_or(|s| s.is_empty()) {
                            return Err(AppError::Config(format!(
                                "Header operation {:?} for upstream '{}' requires a valid value",
                                header_op.op, upstream.name
                            )));
                        }
                    }
                    HeaderOpType::Remove => {}
                }
            }
        }

        Ok(())
    }

    // 验证名称唯一性
    pub fn validate_name_uniqueness(&self) -> Result<(), AppError> {
        // 验证转发服务名称唯一性
        let mut forward_names = HashSet::new();
        for forward in &self.http_server.forwards {
            if !forward_names.insert(forward.name.as_str()) {
                return Err(AppError::Config(format!(
                    "Forwarding service name '{}' is duplicated",
                    forward.name
                )));
            }
        }

        // 验证上游名称唯一性
        let mut upstream_names = HashSet::new();
        for upstream in &self.upstreams {
            if !upstream_names.insert(upstream.name.as_str()) {
                return Err(AppError::Config(format!(
                    "Upstream name '{}' is duplicated",
                    upstream.name
                )));
            }
        }

        // 验证上游组名称唯一性
        let mut group_names = HashSet::new();
        for group in &self.upstream_groups {
            if !group_names.insert(group.name.as_str()) {
                return Err(AppError::Config(format!(
                    "Upstream group name '{}' is duplicated",
                    group.name
                )));
            }
        }

        Ok(())
    }

    // 验证超时配置
    pub fn validate_timeout_config(
        &self,
        timeout: &TimeoutConfig,
        context: &str,
    ) -> Result<(), AppError> {
        if timeout.connect < http_client_limits::MIN_CONNECT_TIMEOUT
            || timeout.connect > http_client_limits::MAX_CONNECT_TIMEOUT
        {
            return Err(AppError::Config(format!(
                "Connect timeout {}s for {} is out of valid range [{}-{}]s",
                timeout.connect,
                context,
                http_client_limits::MIN_CONNECT_TIMEOUT,
                http_client_limits::MAX_CONNECT_TIMEOUT
            )));
        }
        Ok(())
    }

    // 验证 HTTP 客户端配置
    pub fn validate_http_client_config(
        &self,
        config: &HttpClientConfig,
        context: &str,
    ) -> Result<(), AppError> {
        // 验证连接超时
        if config.timeout.connect < http_client_limits::MIN_CONNECT_TIMEOUT
            || config.timeout.connect > http_client_limits::MAX_CONNECT_TIMEOUT
        {
            return Err(AppError::Config(format!(
                "Connect timeout {}s for {} is out of valid range [{}-{}]s",
                config.timeout.connect,
                context,
                http_client_limits::MIN_CONNECT_TIMEOUT,
                http_client_limits::MAX_CONNECT_TIMEOUT
            )));
        }

        // 验证请求超时
        if config.timeout.request < http_client_limits::MIN_REQUEST_TIMEOUT
            || config.timeout.request > http_client_limits::MAX_REQUEST_TIMEOUT
        {
            return Err(AppError::Config(format!(
                "Request timeout {}s for {} is out of valid range [{}-{}]s",
                config.timeout.request,
                context,
                http_client_limits::MIN_REQUEST_TIMEOUT,
                http_client_limits::MAX_REQUEST_TIMEOUT
            )));
        }

        // 验证空闲连接超时
        if config.timeout.idle < http_client_limits::MIN_IDLE_TIMEOUT
            || config.timeout.idle > http_client_limits::MAX_IDLE_TIMEOUT
        {
            return Err(AppError::Config(format!(
                "Idle connection timeout {}s for {} is out of valid range [{}-{}]s",
                config.timeout.idle,
                context,
                http_client_limits::MIN_IDLE_TIMEOUT,
                http_client_limits::MAX_IDLE_TIMEOUT
            )));
        }

        // 验证TCP Keepalive
        if config.keepalive < http_client_limits::MIN_KEEPALIVE
            || config.keepalive > http_client_limits::MAX_KEEPALIVE
        {
            return Err(AppError::Config(format!(
                "TCP Keepalive {}s for {} is out of valid range [{}-{}]s",
                config.keepalive,
                context,
                http_client_limits::MIN_KEEPALIVE,
                http_client_limits::MAX_KEEPALIVE
            )));
        }

        // 验证重试配置
        if config.retry.enabled {
            if config.retry.attempts < retry_limits::MIN_ATTEMPTS
                || config.retry.attempts > retry_limits::MAX_ATTEMPTS
            {
                return Err(AppError::Config(format!(
                    "Retry attempts {} for {} is out of valid range [{}-{}]",
                    config.retry.attempts,
                    context,
                    retry_limits::MIN_ATTEMPTS,
                    retry_limits::MAX_ATTEMPTS
                )));
            }

            if config.retry.initial < retry_limits::MIN_INITIAL_MS
                || config.retry.initial > retry_limits::MAX_INITIAL_MS
            {
                return Err(AppError::Config(format!(
                    "Initial retry interval {}ms for {} is out of valid range [{}-{}]ms",
                    config.retry.initial,
                    context,
                    retry_limits::MIN_INITIAL_MS,
                    retry_limits::MAX_INITIAL_MS
                )));
            }
        }

        // 验证代理配置
        if config.proxy.enabled {
            if config.proxy.url.is_empty() {
                return Err(AppError::Config(format!(
                    "Proxy URL for {} cannot be empty when proxy is enabled",
                    context
                )));
            }
            if let Err(e) = Url::parse(&config.proxy.url) {
                return Err(AppError::Config(format!(
                    "Proxy URL '{}' is invalid: {}",
                    config.proxy.url, e
                )));
            }
        }

        // 验证流式模式配置
        // 当流式模式启用时，检查请求超时设置是否合理
        if config.stream_mode
            && config.timeout.request < http_client_limits::DEFAULT_REQUEST_TIMEOUT
        {
            // 对于流式响应，建议使用更长的请求超时
            return Err(AppError::Config(format!(
                "Request timeout {}s for {} with stream_mode enabled is too short, recommended minimum is {}s",
                config.timeout.request,
                context,
                http_client_limits::DEFAULT_REQUEST_TIMEOUT
            )));
        }

        Ok(())
    }

    // 验证限流配置
    pub fn validate_rate_limit_config(
        &self,
        config: &RateLimitConfig,
        service_name: &str,
    ) -> Result<(), AppError> {
        if config.per_second < rate_limit_limits::MIN_PER_SECOND
            || config.per_second > rate_limit_limits::MAX_PER_SECOND
        {
            return Err(AppError::Config(format!(
                "Requests per second {} for forwarding service '{}' is out of valid range [{}-{}]",
                config.per_second,
                service_name,
                rate_limit_limits::MIN_PER_SECOND,
                rate_limit_limits::MAX_PER_SECOND
            )));
        }

        if config.burst < rate_limit_limits::MIN_BURST
            || config.burst > rate_limit_limits::MAX_BURST
        {
            return Err(AppError::Config(format!(
                "Burst limit {} for forwarding service '{}' is out of valid range [{}-{}]",
                config.burst,
                service_name,
                rate_limit_limits::MIN_BURST,
                rate_limit_limits::MAX_BURST
            )));
        }

        Ok(())
    }

    // 验证熔断器配置
    pub fn validate_breaker_config(
        &self,
        breaker: &BreakerConfig,
        upstream_name: &str,
    ) -> Result<(), AppError> {
        if breaker.threshold < breaker_limits::MIN_THRESHOLD
            || breaker.threshold > breaker_limits::MAX_THRESHOLD
        {
            return Err(AppError::Config(format!(
                "Upstream '{}' has invalid breaker.threshold ({}), must be between 0.01 and 1.0",
                upstream_name, breaker.threshold
            )));
        }

        if breaker.cooldown < breaker_limits::MIN_COOLDOWN
            || breaker.cooldown > breaker_limits::MAX_COOLDOWN
        {
            return Err(AppError::Config(format!(
                "Upstream '{}' has invalid breaker.cooldown ({}s), must be between 5 and 3600 seconds",
                upstream_name, breaker.cooldown
            )));
        }

        Ok(())
    }
}
