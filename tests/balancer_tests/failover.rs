// tests/balancer/failover.rs

// This module contains tests for the Failover balancer.

use super::common::create_test_managed_upstreams;
use llmproxy::{
    balancer::{FailoverBalancer, LoadBalancer, ManagedUpstream},
    config::{BalanceStrategy, UpstreamRef},
};

#[tokio::test]
async fn test_failover_balancer_creation() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = FailoverBalancer::new(managed_upstreams);
    let upstream = balancer.select_upstream().await.unwrap();
    assert_eq!(upstream.upstream_ref.name, "upstream1");
}

#[tokio::test]
async fn test_failover_balancer_selection_order() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = FailoverBalancer::new(managed_upstreams);
    let first = balancer.select_upstream().await.unwrap();
    assert_eq!(first.upstream_ref.name, "upstream1");
}

#[tokio::test]
async fn test_load_balancer_factory_failover() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer =
        llmproxy::balancer::create_load_balancer(&BalanceStrategy::Failover, managed_upstreams);
    assert_eq!(balancer.as_str(), "failover");
    assert!(balancer.select_upstream().await.is_ok());
}

#[tokio::test]
async fn test_failover_balancer_with_unavailable_upstream() {
    let mut managed_upstreams = vec![
        ManagedUpstream {
            upstream_ref: UpstreamRef {
                name: "unavailable".to_string(),
                weight: 1,
            },
            breaker: None,
        },
        ManagedUpstream {
            upstream_ref: UpstreamRef {
                name: "available".to_string(),
                weight: 1,
            },
            breaker: None,
        },
    ];

    let breaker_config = llmproxy::config::BreakerConfig {
        threshold: 0.5,
        cooldown: 30,
    };
    let breaker = llmproxy::breaker::create_upstream_circuit_breaker(
        "test_upstream".to_string(),
        "test_group".to_string(),
        &breaker_config,
    );

    for _ in 0..10 {
        let _ = breaker
            .call_async(|| async {
                Err::<(), _>(llmproxy::breaker::UpstreamError("test failure".to_string()))
            })
            .await;
    }
    assert!(!breaker.is_call_permitted());

    managed_upstreams[0].breaker = Some(breaker);

    let balancer = FailoverBalancer::new(managed_upstreams);

    let selected = balancer.select_upstream().await.unwrap();
    assert_eq!(selected.upstream_ref.name, "available");
}
