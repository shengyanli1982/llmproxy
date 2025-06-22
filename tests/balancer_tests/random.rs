// tests/balancer/random.rs

// This module contains tests for the Random balancer.
use super::common::create_test_managed_upstreams;
use llmproxy::balancer::{LoadBalancer, RandomBalancer};
use llmproxy::config::BalanceStrategy;

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
