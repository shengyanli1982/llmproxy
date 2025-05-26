use crate::config::{BalanceStrategy, UpstreamRef};
use crate::error::AppError;
use async_trait::async_trait;
use rand::{seq::SliceRandom, thread_rng};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::debug;

/// 负载均衡器特性
#[async_trait]
pub trait LoadBalancer: Send + Sync {
    /// 选择一个上游服务器
    async fn select_upstream(&self) -> Result<&UpstreamRef, AppError>;

    /// 报告服务器失败
    async fn report_failure(&self, upstream: &UpstreamRef);
}

/// 轮询负载均衡器
pub struct RoundRobinBalancer {
    /// 服务器列表
    upstreams: Vec<UpstreamRef>,
    /// 当前索引（原子操作）
    current: AtomicUsize,
}

impl RoundRobinBalancer {
    /// 创建新的轮询负载均衡器
    pub fn new(upstreams: Vec<UpstreamRef>) -> Self {
        Self {
            upstreams,
            current: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl LoadBalancer for RoundRobinBalancer {
    async fn select_upstream(&self) -> Result<&UpstreamRef, AppError> {
        if self.upstreams.is_empty() {
            return Err(AppError::NoUpstreamAvailable);
        }

        let current_index = self.current.fetch_add(1, Ordering::SeqCst) % self.upstreams.len();
        let upstream = &self.upstreams[current_index];
        debug!(
            "RoundRobinBalancer selected upstream: {} ({}), index: {}",
            upstream.name,
            self.upstreams
                .get(current_index)
                .map_or("unknown_address", |u| &u.name),
            current_index
        );
        Ok(upstream)
    }

    async fn report_failure(&self, _upstream: &UpstreamRef) {
        // 轮询策略下不需要特殊处理失败
    }
}

/// 加权轮询负载均衡器
pub struct WeightedRoundRobinBalancer {
    /// 服务器列表，按权重复制
    upstreams: Vec<UpstreamRef>,
    /// 当前索引（原子操作）
    current: AtomicUsize,
}

impl WeightedRoundRobinBalancer {
    /// 创建新的加权轮询负载均衡器
    pub fn new(upstreams: Vec<UpstreamRef>) -> Self {
        // 根据权重复制服务器
        let mut weighted_upstreams = Vec::new();

        for upstream in upstreams {
            // 对于每个服务器，按其权重添加多个副本
            for _ in 0..upstream.weight {
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
    async fn select_upstream(&self) -> Result<&UpstreamRef, AppError> {
        if self.upstreams.is_empty() {
            return Err(AppError::NoUpstreamAvailable);
        }

        let current_index = self.current.fetch_add(1, Ordering::SeqCst) % self.upstreams.len();
        let upstream = &self.upstreams[current_index];
        debug!(
            "WeightedRoundRobinBalancer selected upstream: {} ({}), weight: {}, index: {}",
            upstream.name,
            self.upstreams
                .get(current_index)
                .map_or("unknown_address", |u| &u.name),
            upstream.weight,
            current_index
        );
        Ok(upstream)
    }

    async fn report_failure(&self, _upstream: &UpstreamRef) {
        // 加权轮询策略下不需要特殊处理失败
    }
}

/// 随机负载均衡器
pub struct RandomBalancer {
    /// 服务器列表
    upstreams: Vec<UpstreamRef>,
}

impl RandomBalancer {
    /// 创建新的随机负载均衡器
    pub fn new(upstreams: Vec<UpstreamRef>) -> Self {
        Self { upstreams }
    }
}

#[async_trait]
impl LoadBalancer for RandomBalancer {
    async fn select_upstream(&self) -> Result<&UpstreamRef, AppError> {
        if self.upstreams.is_empty() {
            return Err(AppError::NoUpstreamAvailable);
        }

        let upstream = self
            .upstreams
            .choose(&mut thread_rng())
            .ok_or(AppError::NoUpstreamAvailable)?;
        debug!(
            "RandomBalancer selected upstream: {} ({})",
            upstream.name, upstream.name
        );
        Ok(upstream)
    }

    async fn report_failure(&self, _upstream: &UpstreamRef) {
        // 随机策略下不需要特殊处理失败
    }
}

/// 创建负载均衡器
pub fn create_load_balancer(
    strategy: &BalanceStrategy,
    upstreams: Vec<UpstreamRef>,
) -> Arc<dyn LoadBalancer> {
    match strategy {
        BalanceStrategy::RoundRobin => Arc::new(RoundRobinBalancer::new(upstreams)),
        BalanceStrategy::WeightedRoundRobin => Arc::new(WeightedRoundRobinBalancer::new(upstreams)),
        BalanceStrategy::Random => Arc::new(RandomBalancer::new(upstreams)),
    }
}
