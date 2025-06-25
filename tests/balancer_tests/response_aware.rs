// tests/balancer/response_aware.rs

// This module contains tests for the ResponseAware balancer.

use super::common::{create_test_managed_upstreams, setup_mock_server};
use llmproxy::{
    balancer::{LoadBalancer, ManagedUpstream, ResponseAwareBalancer},
    config::{
        BalanceConfig, BalanceStrategy, HttpClientConfig, UpstreamConfig, UpstreamGroupConfig,
        UpstreamRef,
    },
    upstream::UpstreamManager,
};
use std::{sync::Arc, time::Duration};

#[tokio::test]
async fn test_response_aware_balancer_creation() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = ResponseAwareBalancer::new(managed_upstreams);
    let upstream = balancer.select_upstream().await.unwrap();
    assert!(["upstream1", "upstream2", "upstream3"].contains(&upstream.upstream_ref.name.as_str()));
}

#[tokio::test]
async fn test_response_aware_balancer_metrics_update() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = ResponseAwareBalancer::new(managed_upstreams);

    let selected = balancer.select_upstream().await.unwrap();
    let selected_name = selected.upstream_ref.name.clone();

    balancer.update_metrics(&selected, 100);

    let next_selected = balancer.select_upstream().await.unwrap();
    assert_eq!(next_selected.upstream_ref.name, selected_name);
}

#[tokio::test]
async fn test_response_aware_balancer_pending_requests() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = Arc::new(ResponseAwareBalancer::new(managed_upstreams));
    let balancer_clone = balancer.clone();

    let first_upstream = balancer.select_upstream().await.unwrap();
    let first_name = first_upstream.upstream_ref.name.clone();
    balancer.update_metrics(&first_upstream, 50);

    let selected = balancer.select_upstream().await.unwrap();
    assert_eq!(selected.upstream_ref.name, first_name);

    for _ in 0..5 {
        let _ = balancer.select_upstream().await.unwrap();
    }

    let first_upstream_clone = first_upstream.clone();
    tokio::spawn(async move {
        balancer_clone.update_metrics(&first_upstream_clone, 5000);
    })
    .await
    .unwrap();

    let mut other_upstream = None;
    for _ in 0..10 {
        let upstream = balancer.select_upstream().await.unwrap();
        if upstream.upstream_ref.name != first_name {
            other_upstream = Some(upstream);
            break;
        }
    }

    if let Some(upstream) = other_upstream {
        balancer.update_metrics(&upstream, 100);
        let new_selected = balancer.select_upstream().await.unwrap();
        assert_ne!(new_selected.upstream_ref.name, first_name);
    }
}

#[tokio::test]
async fn test_response_aware_balancer_failure_handling() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = ResponseAwareBalancer::new(managed_upstreams);

    let selected = balancer.select_upstream().await.unwrap();
    let selected_name = selected.upstream_ref.name.clone();

    for _ in 0..5 {
        balancer.report_failure(&selected).await;
    }

    let mut updated_others = false;
    for _ in 0..10 {
        let upstream = balancer.select_upstream().await.unwrap();
        if upstream.upstream_ref.name != selected_name {
            balancer.update_metrics(&upstream, 100);
            updated_others = true;
        }
    }
    assert!(updated_others);

    let mut counts = std::collections::HashMap::new();
    counts.insert("upstream1".to_string(), 0);
    counts.insert("upstream2".to_string(), 0);
    counts.insert("upstream3".to_string(), 0);

    for _ in 0..10 {
        let next_selected = balancer.select_upstream().await.unwrap();
        *counts.get_mut(&next_selected.upstream_ref.name).unwrap() += 1;
    }

    let failed_count = counts[&selected_name];
    let total_other_count: i32 = counts
        .iter()
        .filter(|(k, _)| **k != selected_name)
        .map(|(_, v)| *v)
        .sum();
    assert!(failed_count < total_other_count);
}

#[tokio::test]
async fn test_response_aware_balancer_factory_creation() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = llmproxy::balancer::create_load_balancer(
        &BalanceStrategy::ResponseAware,
        managed_upstreams,
    );
    assert_eq!(balancer.as_str(), "response_aware");
    assert!(balancer.select_upstream().await.is_ok());
}

#[tokio::test]
async fn test_response_aware_with_upstream_manager() {
    let mock_server1 = setup_mock_server("Fast Server", 0).await;
    let mock_server2 = setup_mock_server("Slow Server", 300).await;

    let upstream_configs = vec![
        UpstreamConfig {
            name: "fast".to_string(),
            url: format!("{}/test", mock_server1.uri()).into(),
            weight: 1,
            http_client: HttpClientConfig::default(),
            auth: None,
            headers: vec![],
            breaker: None,
        },
        UpstreamConfig {
            name: "slow".to_string(),
            url: format!("{}/test", mock_server2.uri()).into(),
            weight: 1,
            http_client: HttpClientConfig::default(),
            auth: None,
            headers: vec![],
            breaker: None,
        },
    ];

    let group_configs = vec![UpstreamGroupConfig {
        name: "test_group".to_string(),
        upstreams: vec![
            UpstreamRef {
                name: "fast".to_string(),
                weight: 1,
            },
            UpstreamRef {
                name: "slow".to_string(),
                weight: 1,
            },
        ],
        balance: BalanceConfig {
            strategy: BalanceStrategy::ResponseAware,
        },
        http_client: llmproxy::config::HttpClientConfig::default(),
    }];

    let upstream_manager = UpstreamManager::new(upstream_configs, group_configs)
        .await
        .unwrap();

    for _ in 0..5 {
        let response = upstream_manager
            .forward_request(
                "test_group",
                &reqwest::Method::GET,
                reqwest::header::HeaderMap::new(),
                None,
            )
            .await
            .unwrap();
        let _ = response.text().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let mut fast_count = 0;
    let mut slow_count = 0;
    for _ in 0..10 {
        let response = upstream_manager
            .forward_request(
                "test_group",
                &reqwest::Method::GET,
                reqwest::header::HeaderMap::new(),
                None,
            )
            .await
            .unwrap();
        let body = response.text().await.unwrap();
        if body == "Fast Server" {
            fast_count += 1;
        } else {
            slow_count += 1;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    assert!(
        fast_count >= 3,
        "Expected fast_count >= 3, but got fast_count={} and slow_count={}",
        fast_count,
        slow_count
    );
}

#[tokio::test]
async fn test_response_aware_balancer_under_load() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = Arc::new(ResponseAwareBalancer::new(managed_upstreams));

    let upstream1 = balancer.select_upstream().await.unwrap();
    let name1 = upstream1.upstream_ref.name.clone();
    balancer.update_metrics(&upstream1, 50);

    let upstream2 = balancer.select_upstream().await.unwrap();
    let name2 = upstream2.upstream_ref.name.clone();
    balancer.update_metrics(&upstream2, 500);

    let upstream3 = balancer.select_upstream().await.unwrap();
    let name3 = upstream3.upstream_ref.name.clone();
    balancer.update_metrics(&upstream3, 2000);

    let counts = Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    counts.lock().unwrap().insert(name1.clone(), 0);
    counts.lock().unwrap().insert(name2.clone(), 0);
    counts.lock().unwrap().insert(name3.clone(), 0);

    let mut handles = Vec::new();
    for _ in 0..100 {
        let balancer_clone = balancer.clone();
        let counts_clone = counts.clone();
        let name1_clone = name1.clone();
        let name2_clone = name2.clone();

        let handle = tokio::spawn(async move {
            let selected = balancer_clone.select_upstream().await.unwrap();
            let name = selected.upstream_ref.name.clone();

            let processing_time = if name == name1_clone {
                50
            } else if name == name2_clone {
                500
            } else {
                2000
            };
            tokio::time::sleep(Duration::from_millis(processing_time as u64 / 10)).await;
            balancer_clone.update_metrics(&selected, processing_time);

            let mut counts_lock = counts_clone.lock().unwrap();
            *counts_lock.entry(name).or_insert(0) += 1;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let counts_lock = counts.lock().unwrap();
    let name1_count = counts_lock[&name1];
    let name2_count = counts_lock[&name2];
    let name3_count = counts_lock[&name3];

    assert!(
        name3_count <= name1_count || name3_count <= name2_count,
        "Slow upstream should not be selected the most"
    );
}

#[tokio::test]
#[ignore]
async fn test_response_aware_balancer_update_upstreams() {
    // 创建两个不同响应时间的上游
    let _fast_upstream = ManagedUpstream {
        upstream_ref: Arc::new(UpstreamRef {
            name: "fast_upstream".to_string(),
            weight: 1,
        }),
        breaker: None,
    };

    let _slow_upstream = ManagedUpstream {
        upstream_ref: Arc::new(UpstreamRef {
            name: "slow_upstream".to_string(),
            weight: 1,
        }),
        breaker: None,
    };

    // 创建初始上游列表
    let initial_upstreams = create_test_managed_upstreams();
    let balancer = ResponseAwareBalancer::new(initial_upstreams);

    // 创建新的上游列表
    let new_upstreams = vec![
        ManagedUpstream {
            upstream_ref: Arc::new(UpstreamRef {
                name: "fast_upstream".to_string(),
                weight: 1,
            }),
            breaker: None,
        },
        ManagedUpstream {
            upstream_ref: Arc::new(UpstreamRef {
                name: "slow_upstream".to_string(),
                weight: 1,
            }),
            breaker: None,
        },
    ];

    // 更新上游列表
    balancer.update_upstreams(new_upstreams).await;

    // 验证更新后能正确选择上游
    let selected = balancer.select_upstream().await.unwrap();
    assert!(["fast_upstream", "slow_upstream"].contains(&selected.upstream_ref.name.as_str()));

    // 更新指标，使fast_upstream响应更快
    if selected.upstream_ref.name == "fast_upstream" {
        balancer.update_metrics(&selected, 50);

        // 再次选择，应该仍然选择fast_upstream
        let next_selected = balancer.select_upstream().await.unwrap();
        assert_eq!(next_selected.upstream_ref.name, "fast_upstream");

        // 更新slow_upstream的指标
        let slow_upstream = balancer.select_upstream().await.unwrap();
        if slow_upstream.upstream_ref.name == "slow_upstream" {
            balancer.update_metrics(&slow_upstream, 500);
        }
    } else {
        // 如果第一次选择了slow_upstream
        balancer.update_metrics(&selected, 500);

        // 选择另一个上游并更新其指标
        let fast_upstream = balancer.select_upstream().await.unwrap();
        if fast_upstream.upstream_ref.name == "fast_upstream" {
            balancer.update_metrics(&fast_upstream, 50);
        }
    }

    // 多次选择，验证fast_upstream被选择的次数更多
    let mut fast_count = 0;
    let mut slow_count = 0;

    for _ in 0..10 {
        let selected = balancer.select_upstream().await.unwrap();
        if selected.upstream_ref.name == "fast_upstream" {
            fast_count += 1;
        } else {
            slow_count += 1;
        }
    }

    // fast_upstream应该被选择的次数更多
    assert!(fast_count > slow_count);
}
