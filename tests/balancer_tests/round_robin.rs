// tests/balancer/round_robin.rs

// This module contains tests for the RoundRobin and WeightedRoundRobin balancers.

use super::common::create_test_managed_upstreams;
use llmproxy::balancer::{
    LoadBalancer, ManagedUpstream, RoundRobinBalancer, WeightedRoundRobinBalancer,
};
use llmproxy::config::UpstreamRef;
use std::sync::Arc;

#[tokio::test]
async fn test_round_robin_balancer_creation() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = RoundRobinBalancer::new(managed_upstreams);
    let upstream = balancer.select_upstream().await.unwrap();
    assert_eq!(upstream.upstream_ref.name, "upstream1");
}

#[tokio::test]
async fn test_round_robin_balancer_selection() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = RoundRobinBalancer::new(managed_upstreams);

    let first = balancer.select_upstream().await.unwrap();
    assert_eq!(first.upstream_ref.name, "upstream1");
    let second = balancer.select_upstream().await.unwrap();
    assert_eq!(second.upstream_ref.name, "upstream2");
    let third = balancer.select_upstream().await.unwrap();
    assert_eq!(third.upstream_ref.name, "upstream3");
    let fourth = balancer.select_upstream().await.unwrap();
    assert_eq!(fourth.upstream_ref.name, "upstream1");
}

#[tokio::test]
async fn test_weighted_round_robin_balancer_creation() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = WeightedRoundRobinBalancer::new(managed_upstreams);
    let upstream = balancer.select_upstream().await.unwrap();
    assert!(["upstream1", "upstream2", "upstream3"].contains(&upstream.upstream_ref.name.as_str()));
}

#[tokio::test]
async fn test_weighted_round_robin_balancer_distribution() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = WeightedRoundRobinBalancer::new(managed_upstreams);

    let mut counts = std::collections::HashMap::new();
    counts.insert("upstream1".to_string(), 0);
    counts.insert("upstream2".to_string(), 0);
    counts.insert("upstream3".to_string(), 0);

    const ITERATIONS: usize = 12;
    for _ in 0..ITERATIONS {
        let selected = balancer.select_upstream().await.unwrap();
        *counts.get_mut(&selected.upstream_ref.name).unwrap() += 1;
    }

    assert_eq!(counts["upstream1"], 2);
    assert_eq!(counts["upstream2"], 4);
    assert_eq!(counts["upstream3"], 6);
}

#[tokio::test]
async fn test_round_robin_factory() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = llmproxy::balancer::create_load_balancer(
        &llmproxy::config::BalanceStrategy::RoundRobin,
        managed_upstreams,
    );
    assert!(balancer.select_upstream().await.is_ok());
}

#[tokio::test]
async fn test_weighted_round_robin_factory() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = llmproxy::balancer::create_load_balancer(
        &llmproxy::config::BalanceStrategy::WeightedRoundRobin,
        managed_upstreams,
    );
    assert!(balancer.select_upstream().await.is_ok());
}

#[tokio::test]
async fn test_round_robin_balancer_update_upstreams() {
    // 创建初始上游列表
    let initial_upstreams = create_test_managed_upstreams();
    let balancer = RoundRobinBalancer::new(initial_upstreams);

    // 初始状态下应该轮询选择
    let first = balancer.select_upstream().await.unwrap();
    let second = balancer.select_upstream().await.unwrap();
    assert_ne!(first.upstream_ref.name, second.upstream_ref.name);

    // 创建新的上游列表
    let new_upstreams = vec![ManagedUpstream {
        upstream_ref: Arc::new(UpstreamRef {
            name: "new_upstream".to_string(),
            weight: 1,
        }),
        breaker: None,
    }];

    // 更新上游列表
    balancer.update_upstreams(new_upstreams).await;

    // 验证更新后的状态
    let updated = balancer.select_upstream().await.unwrap();
    assert_eq!(updated.upstream_ref.name, "new_upstream");
}

#[tokio::test]
async fn test_weighted_round_robin_balancer_update_upstreams() {
    // 创建初始上游列表
    let initial_upstreams = create_test_managed_upstreams();
    let balancer = WeightedRoundRobinBalancer::new(initial_upstreams);

    // 创建新的上游列表，具有不同的权重
    let new_upstreams = vec![
        ManagedUpstream {
            upstream_ref: Arc::new(UpstreamRef {
                name: "upstream1".to_string(),
                weight: 1,
            }),
            breaker: None,
        },
        ManagedUpstream {
            upstream_ref: Arc::new(UpstreamRef {
                name: "upstream2".to_string(),
                weight: 3,
            }),
            breaker: None,
        },
    ];

    // 更新上游列表
    balancer.update_upstreams(new_upstreams).await;

    // 验证更新后的状态，权重更高的上游应该被选择更多次
    let mut upstream1_count = 0;
    let mut upstream2_count = 0;

    for _ in 0..20 {
        let selected = balancer.select_upstream().await.unwrap();
        if selected.upstream_ref.name == "upstream1" {
            upstream1_count += 1;
        } else if selected.upstream_ref.name == "upstream2" {
            upstream2_count += 1;
        }
    }

    // upstream2的权重是upstream1的3倍，所以应该被选择更多次
    assert!(upstream2_count > upstream1_count);
}
