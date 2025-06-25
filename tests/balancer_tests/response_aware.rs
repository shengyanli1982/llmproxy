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
async fn test_response_aware_balancer_update_upstreams() {
    // 启动两个模拟服务器，分别表示快速和慢速服务
    let mock_server_fast = setup_mock_server("Fast Server", 5).await;
    let mock_server_slow = setup_mock_server("Slow Server", 500).await;

    // 创建初始上游列表
    let initial_upstreams = create_test_managed_upstreams();
    let balancer = Arc::new(ResponseAwareBalancer::new(initial_upstreams));

    // 验证初始状态
    let initial_upstream = balancer.select_upstream().await.unwrap();
    assert!(["upstream1", "upstream2", "upstream3"]
        .contains(&initial_upstream.upstream_ref.name.as_str()));

    // 为初始上游设置一些指标数据
    for _ in 0..5 {
        let selected = balancer.select_upstream().await.unwrap();
        balancer.update_metrics(&selected, 100);
    }

    // 创建新的上游列表，基于模拟服务器
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
    balancer.update_upstreams(new_upstreams.clone()).await;

    // 1. 验证更新后只能选择新的上游
    let selected = balancer.select_upstream().await.unwrap();
    assert!(["fast_upstream", "slow_upstream"].contains(&selected.upstream_ref.name.as_str()));

    // 2. 验证指标已重置 - 通过观察选择模式
    // 创建reqwest客户端
    let client = reqwest::Client::new();

    // 向两个服务器多次发送请求，以建立更可靠的性能指标
    for _ in 0..3 {
        // 向快速服务器发送请求
        let fast_upstream = if selected.upstream_ref.name == "fast_upstream" {
            selected.clone()
        } else {
            balancer.select_upstream().await.unwrap()
        };

        if fast_upstream.upstream_ref.name == "fast_upstream" {
            let start = std::time::Instant::now();
            let response = client
                .get(&format!("{}/test", mock_server_fast.uri()))
                .send()
                .await
                .unwrap();
            let _ = response.text().await.unwrap();
            let duration = start.elapsed().as_millis() as usize;

            // 更新指标
            balancer.update_metrics(&fast_upstream, duration);
        }

        // 向慢速服务器发送请求
        let slow_upstream = balancer.select_upstream().await.unwrap();
        if slow_upstream.upstream_ref.name == "slow_upstream" {
            let start = std::time::Instant::now();
            let response = client
                .get(&format!("{}/test", mock_server_slow.uri()))
                .send()
                .await
                .unwrap();
            let _ = response.text().await.unwrap();
            let duration = start.elapsed().as_millis() as usize;

            // 更新指标
            balancer.update_metrics(&slow_upstream, duration);
        }

        // 等待一小段时间，让负载均衡器处理这些指标
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // 3. 多次选择，验证快速上游被更频繁地选择
    let mut fast_count = 0;
    let mut slow_count = 0;

    for _ in 0..20 {
        let selected = balancer.select_upstream().await.unwrap();
        if selected.upstream_ref.name == "fast_upstream" {
            fast_count += 1;
        } else {
            slow_count += 1;
        }
        // 等待一小段时间，避免过快选择
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // 由于性能差异，快速服务器应该被选择的次数更多
    assert!(
        fast_count > slow_count,
        "Expected fast_count > slow_count, but got fast_count={} and slow_count={}",
        fast_count,
        slow_count
    );

    // 4. 测试再次更新上游列表
    // 创建第三个模拟服务器（最快的一个）
    let mock_server_fastest = setup_mock_server("Fastest Server", 2).await;

    // 再次更新上游列表，添加一个新的最快上游
    let newest_upstreams = vec![
        ManagedUpstream {
            upstream_ref: Arc::new(UpstreamRef {
                name: "fastest_upstream".to_string(),
                weight: 1,
            }),
            breaker: None,
        },
        ManagedUpstream {
            upstream_ref: Arc::new(UpstreamRef {
                name: "slow_upstream".to_string(), // 保留之前的慢速上游
                weight: 1,
            }),
            breaker: None,
        },
    ];

    // 更新上游列表
    balancer.update_upstreams(newest_upstreams).await;

    // 验证更新后的列表是否正确
    let selected = balancer.select_upstream().await.unwrap();
    assert!(["fastest_upstream", "slow_upstream"].contains(&selected.upstream_ref.name.as_str()));
    assert!(!["fast_upstream"].contains(&selected.upstream_ref.name.as_str()));

    // 多次更新指标，建立更可靠的性能数据
    for _ in 0..5 {
        // 获取并测试fastest_upstream
        let fastest_upstream = if selected.upstream_ref.name == "fastest_upstream" {
            selected.clone()
        } else {
            balancer.select_upstream().await.unwrap()
        };

        if fastest_upstream.upstream_ref.name == "fastest_upstream" {
            let start = std::time::Instant::now();
            let response = client
                .get(&format!("{}/test", mock_server_fastest.uri()))
                .send()
                .await
                .unwrap();
            let _ = response.text().await.unwrap();
            let duration = start.elapsed().as_millis() as usize;

            // 更新指标
            balancer.update_metrics(&fastest_upstream, duration);
        }

        // 获取并测试slow_upstream
        let slow_upstream = balancer.select_upstream().await.unwrap();
        if slow_upstream.upstream_ref.name == "slow_upstream" {
            let start = std::time::Instant::now();
            let response = client
                .get(&format!("{}/test", mock_server_slow.uri()))
                .send()
                .await
                .unwrap();
            let _ = response.text().await.unwrap();
            let duration = start.elapsed().as_millis() as usize;

            // 更新指标
            balancer.update_metrics(&slow_upstream, duration);
        }

        // 等待一小段时间，让负载均衡器处理指标
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // 测试选择频率
    let mut fastest_count = 0;
    let mut slow_count_after_update = 0;

    for _ in 0..20 {
        let selected = balancer.select_upstream().await.unwrap();
        if selected.upstream_ref.name == "fastest_upstream" {
            fastest_count += 1;
        } else {
            slow_count_after_update += 1;
        }
        // 避免过快选择
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    assert!(
        fastest_count > slow_count_after_update,
        "Expected fastest_count > slow_count_after_update, but got fastest_count={} and slow_count={}",
        fastest_count,
        slow_count_after_update
    );

    // 5. 测试并发更新和请求
    // 创建另一个平衡器
    let balancer_clone = balancer.clone();

    // 创建包含三个上游的新列表
    let concurrent_upstreams = vec![
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
                weight: 1,
            }),
            breaker: None,
        },
        ManagedUpstream {
            upstream_ref: Arc::new(UpstreamRef {
                name: "upstream3".to_string(),
                weight: 1,
            }),
            breaker: None,
        },
    ];

    // 创建两个任务：一个进行更新，一个进行选择
    let update_task = tokio::spawn(async move {
        balancer_clone.update_upstreams(concurrent_upstreams).await;
    });

    // 为最后的验证再创建一个克隆
    let balancer_final = balancer.clone();

    let select_task = tokio::spawn(async move {
        // 尝试在更新发生时进行10次选择
        for _ in 0..10 {
            let _ = balancer.select_upstream().await;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });

    // 等待两个任务完成
    let _ = tokio::join!(update_task, select_task);

    // 验证最终状态是一致的
    let final_upstream = balancer_final.select_upstream().await.unwrap();
    assert!(["upstream1", "upstream2", "upstream3"]
        .contains(&final_upstream.upstream_ref.name.as_str()));
}
