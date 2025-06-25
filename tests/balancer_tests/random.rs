// tests/balancer/random.rs

// This module contains tests for the Random balancer.
use super::common::create_test_managed_upstreams;
use llmproxy::balancer::{LoadBalancer, ManagedUpstream, RandomBalancer};
use llmproxy::config::BalanceStrategy;
use llmproxy::config::UpstreamRef;
use std::sync::Arc;

#[tokio::test]
async fn test_random_balancer() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = RandomBalancer::new(managed_upstreams);

    let upstream = balancer.select_upstream().await.unwrap();
    assert!(["upstream1", "upstream2", "upstream3"].contains(&upstream.upstream_ref.name.as_str()));
}

#[tokio::test]
async fn test_random_balancer_factory() {
    let managed_upstreams = create_test_managed_upstreams();
    let random =
        llmproxy::balancer::create_load_balancer(&BalanceStrategy::Random, managed_upstreams);
    assert!(random.select_upstream().await.is_ok());
}

#[tokio::test]
async fn test_random_balancer_update_upstreams() {
    // 创建初始上游列表
    let initial_upstreams = create_test_managed_upstreams();
    let balancer = RandomBalancer::new(initial_upstreams);

    // 创建新的上游列表
    let new_upstreams = vec![ManagedUpstream {
        upstream_ref: Arc::new(UpstreamRef {
            name: "new_random_upstream".to_string(),
            weight: 1,
        }),
        breaker: None,
    }];

    // 更新上游列表
    balancer.update_upstreams(new_upstreams).await;

    // 验证更新后的状态
    let updated = balancer.select_upstream().await.unwrap();
    assert_eq!(updated.upstream_ref.name, "new_random_upstream");
}
