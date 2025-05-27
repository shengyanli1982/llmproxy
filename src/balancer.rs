use crate::breaker::UpstreamCircuitBreaker;
use crate::config::{BalanceStrategy, UpstreamRef};
use crate::error::AppError;
use async_trait::async_trait;
use rand::{seq::SliceRandom, thread_rng};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::debug;

/// 托管上游结构体，封装上游引用及其关联的熔断器
#[derive(Clone)]
pub struct ManagedUpstream {
    /// 上游引用
    pub upstream_ref: UpstreamRef,
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
}

// 上游健康检查帮助函数
#[inline(always)]
fn is_upstream_healthy(managed_upstream: &ManagedUpstream) -> bool {
    match &managed_upstream.breaker {
        Some(breaker) if !breaker.is_call_permitted() => {
            // 熔断器开启，上游不健康
            debug!(
                "Skipping upstream: {} (circuit breaker open)",
                managed_upstream.upstream_ref.name
            );
            false
        }
        _ => true,
    }
}

// 轮询负载均衡器
pub struct RoundRobinBalancer {
    // 服务器列表
    upstreams: Vec<ManagedUpstream>,
    // 当前索引（原子操作）
    current: AtomicUsize,
}

impl RoundRobinBalancer {
    // 创建新的轮询负载均衡器
    pub fn new(upstreams: Vec<ManagedUpstream>) -> Self {
        Self {
            upstreams,
            current: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl LoadBalancer for RoundRobinBalancer {
    async fn select_upstream(&self) -> Result<&ManagedUpstream, AppError> {
        let len = self.upstreams.len();
        if len == 0 {
            return Err(AppError::NoUpstreamAvailable);
        }

        // 如果只有一个上游，直接检查它
        if len == 1 {
            return if is_upstream_healthy(&self.upstreams[0]) {
                Ok(&self.upstreams[0])
            } else {
                Err(AppError::NoHealthyUpstreamAvailable)
            };
        }

        // 尝试所有上游，找到一个健康的
        let start_index = self.current.fetch_add(1, Ordering::SeqCst) % len;

        for i in 0..len {
            let index = (start_index + i) % len;
            let managed_upstream = &self.upstreams[index];

            if is_upstream_healthy(managed_upstream) {
                debug!(
                    "RoundRobinBalancer selected upstream: {}, index: {}",
                    managed_upstream.upstream_ref.name, index
                );
                return Ok(managed_upstream);
            }
        }

        // 所有上游的熔断器都开启
        debug!("All upstreams have open circuit breakers");
        Err(AppError::NoHealthyUpstreamAvailable)
    }

    async fn report_failure(&self, _upstream: &ManagedUpstream) {
        // 轮询策略下不需要特殊处理失败
    }
}

// 加权轮询负载均衡器
pub struct WeightedRoundRobinBalancer {
    // 服务器列表，按权重复制
    upstreams: Vec<ManagedUpstream>,
    // 当前索引（原子操作）
    current: AtomicUsize,
}

impl WeightedRoundRobinBalancer {
    // 创建新的加权轮询负载均衡器
    pub fn new(upstreams: Vec<ManagedUpstream>) -> Self {
        // 预先计算所需的容量以避免重新分配
        let total_capacity = upstreams
            .iter()
            .map(|u| u.upstream_ref.weight as usize)
            .sum();

        // 根据权重复制服务器
        let mut weighted_upstreams = Vec::with_capacity(total_capacity);

        for upstream in upstreams {
            // 对于每个服务器，按其权重添加多个副本
            let weight = upstream.upstream_ref.weight;
            weighted_upstreams.push(upstream.clone());

            // 从第二个开始添加剩余的副本
            for _ in 1..weight {
                weighted_upstreams.push(upstream.clone());
            }
        }

        Self {
            upstreams: weighted_upstreams,
            current: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl LoadBalancer for WeightedRoundRobinBalancer {
    async fn select_upstream(&self) -> Result<&ManagedUpstream, AppError> {
        let len = self.upstreams.len();
        if len == 0 {
            return Err(AppError::NoUpstreamAvailable);
        }

        // 如果只有一个上游，直接检查它
        if len == 1 {
            return if is_upstream_healthy(&self.upstreams[0]) {
                Ok(&self.upstreams[0])
            } else {
                Err(AppError::NoHealthyUpstreamAvailable)
            };
        }

        // 尝试所有上游，找到一个健康的
        let start_index = self.current.fetch_add(1, Ordering::SeqCst) % len;

        for i in 0..len {
            let index = (start_index + i) % len;
            let managed_upstream = &self.upstreams[index];

            if is_upstream_healthy(managed_upstream) {
                debug!(
                    "WeightedRoundRobinBalancer selected upstream: {}, weight: {}, index: {}",
                    managed_upstream.upstream_ref.name, managed_upstream.upstream_ref.weight, index
                );
                return Ok(managed_upstream);
            }
        }

        // 所有上游的熔断器都开启
        debug!("All upstreams have open circuit breakers");
        Err(AppError::NoHealthyUpstreamAvailable)
    }

    async fn report_failure(&self, _upstream: &ManagedUpstream) {
        // 加权轮询策略下不需要特殊处理失败
    }
}

// 随机负载均衡器
pub struct RandomBalancer {
    // 服务器列表
    upstreams: Vec<ManagedUpstream>,
}

impl RandomBalancer {
    // 创建新的随机负载均衡器
    pub fn new(upstreams: Vec<ManagedUpstream>) -> Self {
        Self { upstreams }
    }
}

#[async_trait]
impl LoadBalancer for RandomBalancer {
    async fn select_upstream(&self) -> Result<&ManagedUpstream, AppError> {
        if self.upstreams.is_empty() {
            return Err(AppError::NoUpstreamAvailable);
        }

        // 如果只有一个上游，直接检查它
        if self.upstreams.len() == 1 {
            return if is_upstream_healthy(&self.upstreams[0]) {
                Ok(&self.upstreams[0])
            } else {
                Err(AppError::NoHealthyUpstreamAvailable)
            };
        }

        // 尝试快速路径：随机选择几次，看是否能找到健康的上游
        let mut rng = thread_rng();
        for _ in 0..3 {
            // 尝试最多3次随机选择
            if let Some(upstream) = self.upstreams.choose(&mut rng) {
                if is_upstream_healthy(upstream) {
                    debug!(
                        "RandomBalancer selected upstream: {}",
                        upstream.upstream_ref.name
                    );
                    return Ok(upstream);
                }
            }
        }

        // 如果随机选择失败，创建健康上游列表
        let healthy_upstreams: Vec<&ManagedUpstream> = self
            .upstreams
            .iter()
            .filter(|upstream| is_upstream_healthy(upstream))
            .collect();

        // 如果没有健康的上游，返回错误
        if healthy_upstreams.is_empty() {
            debug!("All upstreams have open circuit breakers");
            return Err(AppError::NoHealthyUpstreamAvailable);
        }

        // 随机选择一个健康的上游
        let upstream = healthy_upstreams
            .choose(&mut rng)
            .expect("Should have at least one healthy upstream");

        debug!(
            "RandomBalancer selected upstream: {}",
            upstream.upstream_ref.name
        );

        Ok(*upstream)
    }

    async fn report_failure(&self, _upstream: &ManagedUpstream) {
        // 随机策略下不需要特殊处理失败
    }
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
    }
}
