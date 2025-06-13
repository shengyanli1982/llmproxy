use once_cell::sync::Lazy;
use prometheus::{CounterVec, HistogramOpts, HistogramVec, Opts, Registry};

// 应用指标
pub struct Metrics {
    registry: Registry,
    // 上游请求计数
    upstream_requests_total: CounterVec,
    // 上游请求耗时
    upstream_duration_seconds: HistogramVec,
    // 上游错误计数
    upstream_errors_total: CounterVec,
    // HTTP请求计数
    http_requests_total: CounterVec,
    // HTTP请求耗时
    http_request_duration_seconds: HistogramVec,
    // HTTP请求错误计数
    http_request_errors_total: CounterVec,
    // 限流计数
    ratelimit_total: CounterVec,
    // 熔断器状态变化计数
    circuitbreaker_state_changes_total: CounterVec,
    // 熔断器调用结果计数
    circuitbreaker_calls_total: CounterVec,
}

impl Metrics {
    // 创建新的指标收集器
    fn new() -> Self {
        let registry = Registry::new();

        // 上游请求计数
        let upstream_requests_total = CounterVec::new(
            Opts::new(
                "llmproxy_upstream_requests_total",
                "Total number of requests forwarded to upstream services.",
            ),
            &["group", "upstream"],
        )
        .unwrap();

        // 上游请求耗时
        let upstream_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "llmproxy_upstream_duration_seconds",
                "The latency of requests forwarded to upstream services, in seconds.",
            )
            .buckets(vec![
                0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 20.0, 30.0, 60.0,
            ]),
            &["group", "upstream"],
        )
        .unwrap();

        // 上游错误计数
        let upstream_errors_total = CounterVec::new(
            Opts::new(
                "llmproxy_upstream_errors_total",
                "Total number of errors encountered when making requests to upstream services.",
            ),
            &["error", "group", "upstream"],
        )
        .unwrap();

        // HTTP请求计数
        let http_requests_total = CounterVec::new(
            Opts::new(
                "llmproxy_http_requests_total",
                "Total number of incoming HTTP requests received by the proxy.",
            ),
            &["forward", "method"],
        )
        .unwrap();

        // HTTP请求耗时
        let http_request_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "llmproxy_http_request_duration_seconds",
                "The latency of incoming HTTP requests, from the moment they are received until a response is sent, in seconds.",
            )
            .buckets(vec![
                0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 20.0, 30.0, 60.0,
            ]),
            &["forward", "method"],
        )
        .unwrap();

        // HTTP请求错误计数
        let http_request_errors_total = CounterVec::new(
            Opts::new(
                "llmproxy_http_request_errors_total",
                "Total number of errors encountered while processing incoming HTTP requests (e.g., client errors, proxy errors).",
            ),
            &["forward", "error", "status"],
        )
        .unwrap();

        // 限流计数
        let ratelimit_total = CounterVec::new(
            Opts::new(
                "llmproxy_ratelimit_total",
                "Total number of requests that were rejected due to rate limiting.",
            ),
            &["forward"],
        )
        .unwrap();

        // 熔断器状态变化计数
        let circuitbreaker_state_changes_total = CounterVec::new(
            Opts::new(
                "llmproxy_circuitbreaker_state_changes_total",
                "Total number of state changes for the circuit breaker.",
            ),
            &["group", "upstream", "from", "to"],
        )
        .unwrap();

        // 熔断器调用结果计数
        let circuitbreaker_calls_total = CounterVec::new(
            Opts::new(
                "llmproxy_circuitbreaker_calls_total",
                "Total number of calls to the circuit breaker.",
            ),
            &["group", "upstream", "result"],
        )
        .unwrap();

        // 注册指标
        registry
            .register(Box::new(upstream_requests_total.clone()))
            .unwrap();
        registry
            .register(Box::new(upstream_duration_seconds.clone()))
            .unwrap();
        registry
            .register(Box::new(upstream_errors_total.clone()))
            .unwrap();
        registry
            .register(Box::new(http_requests_total.clone()))
            .unwrap();
        registry
            .register(Box::new(http_request_duration_seconds.clone()))
            .unwrap();
        registry
            .register(Box::new(http_request_errors_total.clone()))
            .unwrap();
        registry
            .register(Box::new(ratelimit_total.clone()))
            .unwrap();
        registry
            .register(Box::new(circuitbreaker_state_changes_total.clone()))
            .unwrap();
        registry
            .register(Box::new(circuitbreaker_calls_total.clone()))
            .unwrap();

        Self {
            registry,
            upstream_requests_total,
            upstream_duration_seconds,
            upstream_errors_total,
            http_requests_total,
            http_request_duration_seconds,
            http_request_errors_total,
            ratelimit_total,
            circuitbreaker_state_changes_total,
            circuitbreaker_calls_total,
        }
    }

    // 获取注册表
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    // 上游请求计数
    pub fn upstream_requests_total(&self) -> &CounterVec {
        &self.upstream_requests_total
    }

    // 上游请求耗时
    pub fn upstream_duration_seconds(&self) -> &HistogramVec {
        &self.upstream_duration_seconds
    }

    // 上游错误计数
    pub fn upstream_errors_total(&self) -> &CounterVec {
        &self.upstream_errors_total
    }

    // HTTP请求计数
    pub fn http_requests_total(&self) -> &CounterVec {
        &self.http_requests_total
    }

    // HTTP请求耗时
    pub fn http_request_duration_seconds(&self) -> &HistogramVec {
        &self.http_request_duration_seconds
    }

    // HTTP请求错误计数
    pub fn http_request_errors_total(&self) -> &CounterVec {
        &self.http_request_errors_total
    }

    // 限流计数
    pub fn ratelimit_total(&self) -> &CounterVec {
        &self.ratelimit_total
    }

    // 熔断器状态变化计数
    pub fn circuitbreaker_state_changes_total(&self) -> &CounterVec {
        &self.circuitbreaker_state_changes_total
    }

    // 熔断器调用结果计数
    pub fn circuitbreaker_calls_total(&self) -> &CounterVec {
        &self.circuitbreaker_calls_total
    }

    // 记录上游请求错误
    pub fn record_upstream_request_error(&self, group: &str, upstream: &str, error_type: &str) {
        self.upstream_errors_total
            .with_label_values(&[error_type, group, upstream])
            .inc();
    }
}

// 全局指标实例
pub static METRICS: Lazy<Metrics> = Lazy::new(Metrics::new);
