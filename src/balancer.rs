pub mod response_aware;
pub mod simple;
pub use response_aware::ResponseAwareBalancer;
pub use simple::{
    FailoverBalancer, RandomBalancer, RoundRobinBalancer, WeightedRoundRobinBalancer,
};

use crate::breaker::UpstreamCircuitBreaker;
use crate::config::{BalanceStrategy, UpstreamRef};
use crate::error::AppError;
use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;
use tracing::debug;

/// 托管上游结构体，封装上游引用及其关联的熔断器
#[derive(Clone)]
pub struct ManagedUpstream {
    /// 上游引用
    pub upstream_ref: Arc<UpstreamRef>,
    /// 熔断器（如果启用）
    pub breaker: Option<Arc<UpstreamCircuitBreaker>>,
}

// 负载均衡器特性
#[async_trait]
pub trait LoadBalancer: Send + Sync {
    // 选择一个上游服务器
    async fn select_upstream(&self) -> Result<&ManagedUpstream, AppError>;

    // 报告服务器失败
    async fn report_failure(&self, upstream: &ManagedUpstream);

    // 获取Any类型引用，用于类型转换
    fn as_any(&self) -> &dyn Any;

    // 获取负载均衡器类型字符串标识
    fn as_str(&self) -> &'static str;
}

// 上游健康检查帮助函数
#[inline(always)]
pub fn is_upstream_healthy(managed_upstream: &ManagedUpstream) -> bool {
    // 如果没有熔断器，则认为上游健康
    if let Some(breaker) = &managed_upstream.breaker {
        if !breaker.is_call_permitted() {
            // 熔断器开启，上游不健康
            debug!(
                "Skipping upstream: {} (circuit breaker open)",
                managed_upstream.upstream_ref.name
            );
            return false;
        }
    }

    // 默认健康
    true
}

// 创建负载均衡器
pub fn create_load_balancer(
    strategy: &BalanceStrategy,
    upstreams: Vec<ManagedUpstream>,
) -> Arc<dyn LoadBalancer> {
    match strategy {
        BalanceStrategy::RoundRobin => Arc::new(RoundRobinBalancer::new(upstreams)),
        BalanceStrategy::WeightedRoundRobin => Arc::new(WeightedRoundRobinBalancer::new(upstreams)),
        BalanceStrategy::Random => Arc::new(RandomBalancer::new(upstreams)),
        BalanceStrategy::ResponseAware => Arc::new(ResponseAwareBalancer::new(upstreams)),
        BalanceStrategy::Failover => Arc::new(FailoverBalancer::new(upstreams)),
    }
}
