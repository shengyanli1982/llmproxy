use crate::balancer::{is_upstream_healthy, LoadBalancer, ManagedUpstream};
use crate::error::AppError;
use crate::r#const::balance_strategy_labels;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use tracing::debug;

// 响应时间感知负载均衡器的固定参数
const SMOOTH_FACTOR: f32 = 0.15; // 较小的平滑因子，适合稳定的大模型环境
const INITIAL_RESPONSE_TIME: usize = 2000; // 初始平均响应时间估计 (毫秒)
const INCLUDE_SUCCESS_RATE: bool = true; // 在得分计算中包含成功率

// 响应时间感知负载均衡器
pub struct ResponseAwareBalancer {
    // 服务器列表
    upstreams: Arc<RwLock<Vec<ManagedUpstream>>>,
    // 当前索引（原子操作）
    current: AtomicUsize,
    // 节点指标
    metrics: Arc<RwLock<Vec<UpstreamMetrics>>>,
    // 名称到索引的映射
    name_to_index: Arc<RwLock<HashMap<String, usize>>>,
}

struct UpstreamMetrics {
    // 平均响应时间 (毫秒)
    response_time: AtomicUsize,
    // 处理中请求数
    pending_requests: AtomicUsize,
    // 成功率 (0-1000, 表示 0-100.0%)
    success_rate: AtomicUsize,
}

impl ResponseAwareBalancer {
    // 创建新的响应时间感知负载均衡器
    pub fn new(upstreams: Vec<ManagedUpstream>) -> Self {
        let metrics = (0..upstreams.len())
            .map(|_| UpstreamMetrics {
                response_time: AtomicUsize::new(INITIAL_RESPONSE_TIME),
                pending_requests: AtomicUsize::new(0),
                success_rate: AtomicUsize::new(1000), // 初始 100% 成功率
            })
            .collect();

        let name_to_index = upstreams
            .iter()
            .enumerate()
            .map(|(i, u)| (u.upstream_ref.name.clone(), i))
            .collect();

        Self {
            upstreams: Arc::new(RwLock::new(upstreams)),
            current: AtomicUsize::new(0),
            metrics: Arc::new(RwLock::new(metrics)),
            name_to_index: Arc::new(RwLock::new(name_to_index)),
        }
    }

    // 查找上游索引
    fn find_upstream_index(&self, upstream: &ManagedUpstream) -> Option<usize> {
        let name_to_index = self.name_to_index.read().unwrap();
        name_to_index.get(&upstream.upstream_ref.name).copied()
    }

    // 更新响应时间和减少待处理请求
    pub fn update_metrics(&self, upstream: &ManagedUpstream, response_time_ms: usize) {
        if let Some(index) = self.find_upstream_index(upstream) {
            let metrics = self.metrics.read().unwrap();
            if index < metrics.len() {
                // 更新响应时间
                let old_time = metrics[index].response_time.load(Ordering::Relaxed);
                let new_time = ((1.0 - SMOOTH_FACTOR as f64) * old_time as f64
                    + SMOOTH_FACTOR as f64 * response_time_ms as f64)
                    as usize;

                metrics[index]
                    .response_time
                    .store(new_time, Ordering::Relaxed);

                // 减少待处理请求计数
                metrics[index]
                    .pending_requests
                    .fetch_sub(1, Ordering::SeqCst);

                // 更新成功率 (成功)
                if INCLUDE_SUCCESS_RATE {
                    self.update_success_rate(index, true);
                }

                debug!(
                    "Updated metrics for {:?}: response_time={}ms, pending={}",
                    upstream.upstream_ref.name,
                    new_time,
                    metrics[index].pending_requests.load(Ordering::Relaxed)
                );
            }
        }
    }

    // 更新成功率
    fn update_success_rate(&self, index: usize, success: bool) {
        let metrics = self.metrics.read().unwrap();
        if index < metrics.len() {
            let old_rate = metrics[index].success_rate.load(Ordering::Relaxed);
            let success_value = if success { 1000 } else { 0 };
            let new_rate = ((1.0 - SMOOTH_FACTOR as f64) * old_rate as f64
                + SMOOTH_FACTOR as f64 * success_value as f64) as usize;

            metrics[index]
                .success_rate
                .store(new_rate, Ordering::Relaxed);
        }
    }
}

#[async_trait]
impl LoadBalancer for ResponseAwareBalancer {
    async fn select_upstream(&self) -> Result<&ManagedUpstream, AppError> {
        let upstreams = self.upstreams.read().unwrap();
        let len = upstreams.len();
        if len == 0 {
            return Err(AppError::NoUpstreamAvailable);
        }

        // 如果只有一个上游，直接检查它
        if len == 1 {
            return if is_upstream_healthy(&upstreams[0]) {
                // 增加待处理请求计数
                let metrics = self.metrics.read().unwrap();
                if !metrics.is_empty() {
                    metrics[0].pending_requests.fetch_add(1, Ordering::SeqCst);
                }
                Ok(&upstreams[0])
            } else {
                Err(AppError::NoHealthyUpstreamAvailable)
            };
        }

        // 计算每个节点的负载分数
        let mut best_score = f64::MAX;
        let mut best_index = 0;
        let mut found_healthy = false;

        // 从当前索引开始，确保公平性
        let start_index = self.current.fetch_add(1, Ordering::SeqCst) % len;

        let metrics = self.metrics.read().unwrap();

        // 遍历所有上游，找到健康的最佳节点
        for i in 0..len {
            let index = (start_index + i) % len;
            let managed_upstream = &upstreams[index];

            if is_upstream_healthy(managed_upstream) {
                found_healthy = true;

                if index < metrics.len() {
                    let resp_time = metrics[index].response_time.load(Ordering::Relaxed) as f64;
                    let pending = metrics[index].pending_requests.load(Ordering::Relaxed) as f64;

                    // 计算得分
                    let mut score = resp_time * (pending + 1.0);

                    // 考虑成功率
                    if INCLUDE_SUCCESS_RATE {
                        let success_rate =
                            metrics[index].success_rate.load(Ordering::Relaxed) as f64 / 1000.0;
                        if success_rate > 0.0 {
                            score *= 1.0 / success_rate;
                        }
                    }

                    if score < best_score {
                        best_score = score;
                        best_index = index;
                    }
                }
            }
        }

        if !found_healthy {
            debug!("All upstreams have open circuit breakers");
            return Err(AppError::NoHealthyUpstreamAvailable);
        }

        // 增加选中节点的待处理请求计数
        if best_index < metrics.len() {
            metrics[best_index]
                .pending_requests
                .fetch_add(1, Ordering::SeqCst);
        }

        debug!(
            "ResponseAwareBalancer selected upstream: {:?}, score: {:.2}",
            upstreams[best_index].upstream_ref.name, best_score
        );

        Ok(&upstreams[best_index])
    }

    async fn report_failure(&self, upstream: &ManagedUpstream) {
        // 处理失败情况，可选择更新成功率
        if INCLUDE_SUCCESS_RATE {
            if let Some(index) = self.find_upstream_index(upstream) {
                // 更新成功率 (失败)
                self.update_success_rate(index, false);

                // 减少待处理请求计数
                let metrics = self.metrics.read().unwrap();
                if index < metrics.len() {
                    metrics[index]
                        .pending_requests
                        .fetch_sub(1, Ordering::SeqCst);
                }
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_str(&self) -> &'static str {
        balance_strategy_labels::RESPONSE_AWARE
    }

    async fn update_upstreams(&self, upstreams: Vec<ManagedUpstream>) {
        // 提前计算capacity以减少内存再分配
        let upstreams_len = upstreams.len();

        // 创建新的指标和映射，使用with_capacity预分配内存
        let mut new_metrics = Vec::with_capacity(upstreams_len);
        for _ in 0..upstreams_len {
            new_metrics.push(UpstreamMetrics {
                response_time: AtomicUsize::new(INITIAL_RESPONSE_TIME),
                pending_requests: AtomicUsize::new(0),
                success_rate: AtomicUsize::new(1000), // 初始 100% 成功率
            });
        }

        // 预分配HashMap容量，避免rehash
        let mut new_name_to_index = HashMap::with_capacity(upstreams_len);

        // 填充新的名称到索引映射
        for (i, u) in upstreams.iter().enumerate() {
            new_name_to_index.insert(u.upstream_ref.name.clone(), i);
        }

        // 更新所有状态
        {
            let mut write_guard_upstreams = self.upstreams.write().unwrap();
            let mut write_guard_metrics = self.metrics.write().unwrap();
            let mut write_guard_mapping = self.name_to_index.write().unwrap();

            *write_guard_upstreams = upstreams;
            *write_guard_metrics = new_metrics;
            *write_guard_mapping = new_name_to_index;
        }

        debug!("ResponseAwareBalancer upstreams and metrics updated successfully");
    }
}
