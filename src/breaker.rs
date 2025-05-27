use crate::{
    config::BreakerConfig,
    error::AppError,
    metrics::METRICS,
    r#const::{breaker_result_labels, breaker_state_labels},
};
use circuitbreaker_rs::{BreakerBuilder, CircuitBreaker, DefaultPolicy, HookRegistry, State};
use std::{error::Error, fmt, sync::Arc, time::Duration};
use tracing::{debug, info, warn};

/// 表示上游服务的错误
#[derive(Debug)]
pub struct UpstreamError(pub String);

impl fmt::Display for UpstreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Upstream error: {}", self.0)
    }
}

impl Error for UpstreamError {}

// 添加从AppError到UpstreamError的转换实现
impl From<AppError> for UpstreamError {
    fn from(error: AppError) -> Self {
        UpstreamError(error.to_string())
    }
}

/// 上游服务熔断器
pub struct UpstreamCircuitBreaker {
    breaker: CircuitBreaker<DefaultPolicy, UpstreamError>,
    name: String,
    group: String,
    url: String,
}

impl UpstreamCircuitBreaker {
    /// 创建一个新的熔断器
    pub fn new(
        name: String,
        group: String,
        url: String,
        threshold: f64,
        cooldown: u64,
    ) -> Arc<Self> {
        // 创建事件钩子
        let hooks = Self::create_hooks(&name, &group, &url);

        // 创建熔断器
        let breaker = BreakerBuilder::<DefaultPolicy, UpstreamError>::default()
            .failure_threshold(threshold)
            .cooldown(Duration::from_secs(cooldown))
            .hooks(hooks)
            .build();

        debug!(
            "Created circuit breaker for upstream '{}' in group '{}' with threshold={}, cooldown={}s",
            name, group, threshold, cooldown
        );

        Arc::new(Self {
            breaker,
            name,
            group,
            url,
        })
    }

    /// 使用熔断器执行异步操作
    pub async fn call_async<F, Fut, T>(
        &self,
        f: F,
    ) -> Result<T, circuitbreaker_rs::BreakerError<UpstreamError>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, UpstreamError>>,
    {
        // 直接使用库提供的 call_async 方法
        self.breaker.call_async(f).await
    }

    /// 检查熔断器当前是否允许调用
    pub fn is_call_permitted(&self) -> bool {
        matches!(
            self.breaker.current_state(),
            State::Closed | State::HalfOpen
        )
    }

    /// 获取熔断器当前状态
    pub fn current_state(&self) -> State {
        self.breaker.current_state()
    }

    /// 创建熔断器事件钩子
    fn create_hooks(name: &str, group: &str, url: &str) -> HookRegistry {
        let hooks = HookRegistry::new();

        // 使用引用而不是克隆字符串
        let name_open = name.to_owned();
        let group_open = group.to_owned();
        let url_open = url.to_owned();

        let name_close = name.to_owned();
        let group_close = group.to_owned();
        let url_close = url.to_owned();

        let name_half = name.to_owned();
        let group_half = group.to_owned();
        let url_half = url.to_owned();

        let name_success = name.to_owned();
        let group_success = group.to_owned();
        let url_success = url.to_owned();

        let name_failure = name.to_owned();
        let group_failure = group.to_owned();
        let url_failure = url.to_owned();

        // 状态转换钩子
        hooks.set_on_open(move || {
            // 记录状态变化指标：从关闭到开启
            METRICS
                .circuitbreaker_state_changes_total()
                .with_label_values(&[
                    &group_open,
                    &name_open,
                    &url_open,
                    breaker_state_labels::CLOSED,
                    breaker_state_labels::OPEN,
                ])
                .inc();

            METRICS
                .circuitbreaker_opened_total()
                .with_label_values(&[&group_open, &name_open, &url_open])
                .inc();

            warn!(
                "Circuit breaker opened for upstream '{}' in group '{}'",
                name_open, group_open
            );
        });

        hooks.set_on_close(move || {
            // 记录状态变化指标：从开启或半开到关闭
            METRICS
                .circuitbreaker_state_changes_total()
                .with_label_values(&[
                    &group_close,
                    &name_close,
                    &url_close,
                    breaker_state_labels::OPEN, // 可能是从开启状态
                    breaker_state_labels::CLOSED,
                ])
                .inc();

            // 也可能是从半开状态转为关闭状态
            METRICS
                .circuitbreaker_state_changes_total()
                .with_label_values(&[
                    &group_close,
                    &name_close,
                    &url_close,
                    breaker_state_labels::HALF_OPEN,
                    breaker_state_labels::CLOSED,
                ])
                .inc();

            METRICS
                .circuitbreaker_closed_total()
                .with_label_values(&[&group_close, &name_close, &url_close])
                .inc();

            info!(
                "Circuit breaker closed for upstream '{}' in group '{}'",
                name_close, group_close
            );
        });

        hooks.set_on_half_open(move || {
            // 记录状态变化指标：从开启到半开
            METRICS
                .circuitbreaker_state_changes_total()
                .with_label_values(&[
                    &group_half,
                    &name_half,
                    &url_half,
                    breaker_state_labels::OPEN,
                    breaker_state_labels::HALF_OPEN,
                ])
                .inc();

            METRICS
                .circuitbreaker_half_opened_total()
                .with_label_values(&[&group_half, &name_half, &url_half])
                .inc();

            info!(
                "Circuit breaker half-opened for upstream '{}' in group '{}'",
                name_half, group_half
            );
        });

        // 成功调用钩子
        hooks.set_on_success(move || {
            METRICS
                .circuitbreaker_calls_total()
                .with_label_values(&[
                    &group_success,
                    &name_success,
                    &url_success,
                    breaker_result_labels::SUCCESS,
                ])
                .inc();
        });

        // 失败调用钩子
        hooks.set_on_failure(move || {
            METRICS
                .circuitbreaker_calls_total()
                .with_label_values(&[
                    &group_failure,
                    &name_failure,
                    &url_failure,
                    breaker_result_labels::FAILURE,
                ])
                .inc();
        });

        hooks
    }
}

/// 创建上游服务熔断器
pub fn create_upstream_circuit_breaker(
    name: String,
    group: String,
    url: String,
    config: &BreakerConfig,
) -> Arc<UpstreamCircuitBreaker> {
    UpstreamCircuitBreaker::new(name, group, url, config.threshold, config.cooldown)
}
