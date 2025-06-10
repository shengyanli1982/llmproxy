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

// 创建共享数据结构，避免多次克隆相同的字符串
#[derive(Clone)]
struct HookData {
    name: String,
    group: String,
    url: Arc<String>,
}

/// 上游服务熔断器
pub struct UpstreamCircuitBreaker {
    breaker: CircuitBreaker<DefaultPolicy, UpstreamError>,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    group: String,
    #[allow(dead_code)]
    url: Arc<String>,
}

impl UpstreamCircuitBreaker {
    /// 创建一个新的熔断器
    pub fn new(
        name: String,
        group: String,
        url: Arc<String>,
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
    #[inline(always)]
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
    fn create_hooks(name: &str, group: &str, url: &Arc<String>) -> HookRegistry {
        // 只克隆一次字符串
        let data = HookData {
            name: name.to_owned(),
            group: group.to_owned(),
            url: url.clone(),
        };

        let hooks = HookRegistry::new();

        // 状态转换钩子 - 开启
        let data_open = data.clone();
        hooks.set_on_open(move || {
            // 记录状态变化指标：从关闭到开启
            METRICS
                .circuitbreaker_state_changes_total()
                .with_label_values(&[
                    &data_open.group,
                    &data_open.name,
                    &data_open.url,
                    breaker_state_labels::CLOSED,
                    breaker_state_labels::OPEN,
                ])
                .inc();

            METRICS
                .circuitbreaker_opened_total()
                .with_label_values(&[&data_open.group, &data_open.name, &data_open.url])
                .inc();

            warn!(
                "Circuit breaker opened for upstream '{}' in group '{}'",
                data_open.name, data_open.group
            );
        });

        // 状态转换钩子 - 关闭
        let data_close = data.clone();
        hooks.set_on_close(move || {
            // 记录状态变化指标：从开启或半开到关闭
            METRICS
                .circuitbreaker_state_changes_total()
                .with_label_values(&[
                    &data_close.group,
                    &data_close.name,
                    &data_close.url,
                    breaker_state_labels::OPEN, // 可能是从开启状态
                    breaker_state_labels::CLOSED,
                ])
                .inc();

            // 也可能是从半开状态转为关闭状态
            METRICS
                .circuitbreaker_state_changes_total()
                .with_label_values(&[
                    &data_close.group,
                    &data_close.name,
                    &data_close.url,
                    breaker_state_labels::HALF_OPEN,
                    breaker_state_labels::CLOSED,
                ])
                .inc();

            METRICS
                .circuitbreaker_closed_total()
                .with_label_values(&[&data_close.group, &data_close.name, &data_close.url])
                .inc();

            info!(
                "Circuit breaker closed for upstream '{}' in group '{}'",
                data_close.name, data_close.group
            );
        });

        // 状态转换钩子 - 半开
        let data_half = data.clone();
        hooks.set_on_half_open(move || {
            // 记录状态变化指标：从开启到半开
            METRICS
                .circuitbreaker_state_changes_total()
                .with_label_values(&[
                    &data_half.group,
                    &data_half.name,
                    &data_half.url,
                    breaker_state_labels::OPEN,
                    breaker_state_labels::HALF_OPEN,
                ])
                .inc();

            METRICS
                .circuitbreaker_half_opened_total()
                .with_label_values(&[&data_half.group, &data_half.name, &data_half.url])
                .inc();

            info!(
                "Circuit breaker half-opened for upstream '{}' in group '{}'",
                data_half.name, data_half.group
            );
        });

        // 成功调用钩子
        let data_success = data.clone();
        hooks.set_on_success(move || {
            METRICS
                .circuitbreaker_calls_total()
                .with_label_values(&[
                    &data_success.group,
                    &data_success.name,
                    &data_success.url,
                    breaker_result_labels::SUCCESS,
                ])
                .inc();
        });

        // 失败调用钩子
        let data_failure = data;
        hooks.set_on_failure(move || {
            METRICS
                .circuitbreaker_calls_total()
                .with_label_values(&[
                    &data_failure.group,
                    &data_failure.name,
                    &data_failure.url,
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
    url: Arc<String>,
    config: &BreakerConfig,
) -> Arc<UpstreamCircuitBreaker> {
    UpstreamCircuitBreaker::new(name, group, url, config.threshold, config.cooldown)
}
