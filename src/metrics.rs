use once_cell::sync::Lazy;
use prometheus::{CounterVec, HistogramOpts, HistogramVec, Opts, Registry};

/// 应用指标
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
}

impl Metrics {
    /// 创建新的指标收集器
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
            ),
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
            &["forward", "method", "path"],
        )
        .unwrap();

        // HTTP请求耗时
        let http_request_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "llmproxy_http_request_duration_seconds",
                "The latency of incoming HTTP requests, from the moment they are received until a response is sent, in seconds.",
            ),
            &["forward", "method", "path"],
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

        Self {
            registry,
            upstream_requests_total,
            upstream_duration_seconds,
            upstream_errors_total,
            http_requests_total,
            http_request_duration_seconds,
            http_request_errors_total,
            ratelimit_total,
        }
    }

    /// 获取注册表
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// 上游请求计数
    pub fn upstream_requests_total(&self) -> &CounterVec {
        &self.upstream_requests_total
    }

    /// 上游请求耗时
    pub fn upstream_duration_seconds(&self) -> &HistogramVec {
        &self.upstream_duration_seconds
    }

    /// 上游错误计数
    pub fn upstream_errors_total(&self) -> &CounterVec {
        &self.upstream_errors_total
    }

    /// HTTP请求计数
    pub fn http_requests_total(&self) -> &CounterVec {
        &self.http_requests_total
    }

    /// HTTP请求耗时
    pub fn http_request_duration_seconds(&self) -> &HistogramVec {
        &self.http_request_duration_seconds
    }

    /// HTTP请求错误计数
    pub fn http_request_errors_total(&self) -> &CounterVec {
        &self.http_request_errors_total
    }

    /// 限流计数
    pub fn ratelimit_total(&self) -> &CounterVec {
        &self.ratelimit_total
    }
}

/// 全局指标实例
pub static METRICS: Lazy<Metrics> = Lazy::new(Metrics::new);
