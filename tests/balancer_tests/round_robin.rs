// tests/balancer/round_robin.rs

// This module contains tests for the RoundRobin and WeightedRoundRobin balancers.

use super::common::create_test_managed_upstreams;
use llmproxy::balancer::{LoadBalancer, RoundRobinBalancer, WeightedRoundRobinBalancer};

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
