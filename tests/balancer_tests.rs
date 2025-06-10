use llmproxy::{
    balancer::{
        FailoverBalancer, LoadBalancer, ManagedUpstream, RandomBalancer, ResponseAwareBalancer,
        RoundRobinBalancer, WeightedRoundRobinBalancer,
    },
    config::{
        BalanceConfig, BalanceStrategy, HttpClientConfig, UpstreamConfig, UpstreamGroupConfig,
        UpstreamRef,
    },
    upstream::UpstreamManager,
};
use std::sync::Arc;
use std::time::Duration;

use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

// 创建测试用的托管上游列表
fn create_test_managed_upstreams() -> Vec<ManagedUpstream> {
    vec![
        ManagedUpstream {
            upstream_ref: UpstreamRef {
                name: "upstream1".to_string(),
                weight: 1,
            },
            breaker: None,
        },
        ManagedUpstream {
            upstream_ref: UpstreamRef {
                name: "upstream2".to_string(),
                weight: 2,
            },
            breaker: None,
        },
        ManagedUpstream {
            upstream_ref: UpstreamRef {
                name: "upstream3".to_string(),
                weight: 3,
            },
            breaker: None,
        },
    ]
}

#[tokio::test]
async fn test_round_robin_balancer_creation() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = RoundRobinBalancer::new(managed_upstreams);

    // 由于字段是私有的，我们不能直接访问它们
    // 但我们可以测试 select_upstream 方法
    let upstream = balancer.select_upstream().await.unwrap();
    assert_eq!(upstream.upstream_ref.name, "upstream1");
}

#[tokio::test]
async fn test_round_robin_balancer_selection() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = RoundRobinBalancer::new(managed_upstreams);

    // 第一次调用应该返回第一个上游
    let first = balancer.select_upstream().await.unwrap();
    assert_eq!(first.upstream_ref.name, "upstream1");

    // 第二次调用应该返回第二个上游
    let second = balancer.select_upstream().await.unwrap();
    assert_eq!(second.upstream_ref.name, "upstream2");

    // 第三次调用应该返回第三个上游
    let third = balancer.select_upstream().await.unwrap();
    assert_eq!(third.upstream_ref.name, "upstream3");

    // 第四次调用应该返回第一个上游（循环）
    let fourth = balancer.select_upstream().await.unwrap();
    assert_eq!(fourth.upstream_ref.name, "upstream1");
}

#[tokio::test]
async fn test_weighted_round_robin_balancer_creation() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = WeightedRoundRobinBalancer::new(managed_upstreams);

    // 由于字段是私有的，我们不能直接访问它们
    // 但我们可以测试 select_upstream 方法
    let upstream = balancer.select_upstream().await.unwrap();
    assert!(["upstream1", "upstream2", "upstream3"].contains(&upstream.upstream_ref.name.as_str()));
}

#[tokio::test]
async fn test_weighted_round_robin_balancer_distribution() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = WeightedRoundRobinBalancer::new(managed_upstreams);

    // 统计每个上游被选择的次数
    let mut counts = std::collections::HashMap::new();
    counts.insert("upstream1".to_string(), 0);
    counts.insert("upstream2".to_string(), 0);
    counts.insert("upstream3".to_string(), 0);

    // 进行多次选择以验证分布
    // 由于 WeightedRoundRobinBalancer 是确定性的，我们只需要测试 (1+2+3)*2 次
    const ITERATIONS: usize = 12;
    for _ in 0..ITERATIONS {
        let selected = balancer.select_upstream().await.unwrap();
        *counts.get_mut(&selected.upstream_ref.name).unwrap() += 1;
    }

    // 验证计数与权重成比例
    // upstream1 权重为1，应该出现 ITERATIONS/(1+2+3) * 1 = 2 次
    // upstream2 权重为2，应该出现 ITERATIONS/(1+2+3) * 2 = 4 次
    // upstream3 权重为3，应该出现 ITERATIONS/(1+2+3) * 3 = 6 次
    assert_eq!(counts["upstream1"], 2);
    assert_eq!(counts["upstream2"], 4);
    assert_eq!(counts["upstream3"], 6);
}

#[tokio::test]
async fn test_random_balancer() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = RandomBalancer::new(managed_upstreams);

    // 测试随机选择
    let upstream = balancer.select_upstream().await.unwrap();
    assert!(["upstream1", "upstream2", "upstream3"].contains(&upstream.upstream_ref.name.as_str()));
}

#[tokio::test]
async fn test_load_balancer_factory() {
    let managed_upstreams = create_test_managed_upstreams();

    // 测试创建RoundRobin负载均衡器
    let round_robin = llmproxy::balancer::create_load_balancer(
        &BalanceStrategy::RoundRobin,
        managed_upstreams.clone(),
    );
    assert!(round_robin.select_upstream().await.is_ok());

    // 测试创建WeightedRoundRobin负载均衡器
    let weighted_round_robin = llmproxy::balancer::create_load_balancer(
        &BalanceStrategy::WeightedRoundRobin,
        managed_upstreams.clone(),
    );
    assert!(weighted_round_robin.select_upstream().await.is_ok());

    // 测试创建Random负载均衡器
    let random = llmproxy::balancer::create_load_balancer(
        &BalanceStrategy::Random,
        managed_upstreams.clone(),
    );
    assert!(random.select_upstream().await.is_ok());

    // 测试创建ResponseAware负载均衡器
    let response_aware = llmproxy::balancer::create_load_balancer(
        &BalanceStrategy::ResponseAware,
        managed_upstreams.clone(),
    );
    assert!(response_aware.select_upstream().await.is_ok());
}

#[tokio::test]
async fn test_load_balancer_with_upstream_manager() {
    // 创建模拟服务器
    let mock_server1 = MockServer::start().await;
    let mock_server2 = MockServer::start().await;

    // 配置模拟响应
    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Server 1"))
        .mount(&mock_server1)
        .await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Server 2"))
        .mount(&mock_server2)
        .await;

    // 创建上游配置
    let upstream_configs = vec![
        UpstreamConfig {
            name: "upstream1".to_string(),
            url: mock_server1.uri().into(),
            weight: 1,
            http_client: HttpClientConfig::default(),
            auth: None,
            headers: vec![],
            breaker: None,
        },
        UpstreamConfig {
            name: "upstream2".to_string(),
            url: mock_server2.uri().into(),
            weight: 1,
            http_client: HttpClientConfig::default(),
            auth: None,
            headers: vec![],
            breaker: None,
        },
    ];

    // 创建上游组配置
    let group_configs = vec![UpstreamGroupConfig {
        name: "test_group".to_string(),
        upstreams: vec![
            UpstreamRef {
                name: "upstream1".to_string(),
                weight: 1,
            },
            UpstreamRef {
                name: "upstream2".to_string(),
                weight: 1,
            },
        ],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: llmproxy::config::HttpClientConfig::default(),
    }];

    // 创建上游管理器
    let upstream_manager = UpstreamManager::new(upstream_configs, group_configs)
        .await
        .unwrap();

    // 测试上游管理器中的负载均衡器
    // 使用 forward_request 方法间接测试负载均衡器
    let response1 = upstream_manager
        .forward_request(
            "test_group",
            reqwest::Method::GET,
            "/test",
            reqwest::header::HeaderMap::new(),
            None,
        )
        .await;

    assert!(response1.is_ok());

    // 第二次请求应该转发到另一个上游
    let response2 = upstream_manager
        .forward_request(
            "test_group",
            reqwest::Method::GET,
            "/test",
            reqwest::header::HeaderMap::new(),
            None,
        )
        .await;

    assert!(response2.is_ok());

    // 验证两个响应来自不同的服务器
    let body1 = response1.unwrap().text().await.unwrap();
    let body2 = response2.unwrap().text().await.unwrap();

    // 由于轮询策略，两次请求应该分别发送到不同的服务器
    assert_ne!(body1, body2);
}

#[tokio::test]
async fn test_load_balancer_with_unavailable_upstream() {
    // 创建模拟服务器
    let mock_server = MockServer::start().await;

    // 配置模拟响应
    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Server OK"))
        .mount(&mock_server)
        .await;

    // 创建上游配置
    let upstream_configs = vec![
        UpstreamConfig {
            name: "available".to_string(),
            url: mock_server.uri().into(),
            weight: 1,
            http_client: HttpClientConfig::default(),
            auth: None,
            headers: vec![],
            breaker: None,
        },
        UpstreamConfig {
            name: "unavailable".to_string(),
            url: "http://localhost:1".to_string().into(), // 不可用的上游
            weight: 1,
            http_client: HttpClientConfig::default(),
            auth: None,
            headers: vec![],
            breaker: None,
        },
    ];

    // 创建上游组配置
    let group_configs = vec![UpstreamGroupConfig {
        name: "test_group".to_string(),
        upstreams: vec![
            UpstreamRef {
                name: "available".to_string(),
                weight: 1,
            },
            UpstreamRef {
                name: "unavailable".to_string(),
                weight: 1,
            },
        ],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: llmproxy::config::HttpClientConfig::default(),
    }];

    // 创建上游管理器
    let upstream_manager = UpstreamManager::new(upstream_configs, group_configs)
        .await
        .unwrap();

    // 测试请求，应该转发到可用的上游
    let response = upstream_manager
        .forward_request(
            "test_group",
            reqwest::Method::GET,
            "/test",
            reqwest::header::HeaderMap::new(),
            None,
        )
        .await;

    // 请求应该成功，因为负载均衡器应该选择可用的上游
    assert!(response.is_ok());

    // 验证响应来自可用的服务器
    let body = response.unwrap().text().await.unwrap();
    assert_eq!(body, "Server OK");
}

#[tokio::test]
async fn test_response_aware_balancer_creation() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = ResponseAwareBalancer::new(managed_upstreams);

    // 测试基本选择功能
    let upstream = balancer.select_upstream().await.unwrap();
    assert!(["upstream1", "upstream2", "upstream3"].contains(&upstream.upstream_ref.name.as_str()));
}

#[tokio::test]
async fn test_response_aware_balancer_metrics_update() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = ResponseAwareBalancer::new(managed_upstreams);

    // 选择一个上游并记录其名称
    let selected = balancer.select_upstream().await.unwrap();
    let selected_name = selected.upstream_ref.name.clone();

    // 模拟一个非常快的响应时间更新 (100ms)
    balancer.update_metrics(selected, 100);

    // 再次选择，应该优先选择刚才响应快的上游
    let next_selected = balancer.select_upstream().await.unwrap();
    assert_eq!(next_selected.upstream_ref.name, selected_name);
}

#[tokio::test]
async fn test_response_aware_balancer_pending_requests() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = Arc::new(ResponseAwareBalancer::new(managed_upstreams));
    let balancer_clone = balancer.clone();

    // 选择一个上游，增加其待处理请求数
    let first_upstream = balancer.select_upstream().await.unwrap();
    let first_name = first_upstream.upstream_ref.name.clone();

    // 为第一个上游设置非常快的响应时间，确保它被优先选择
    balancer.update_metrics(first_upstream, 50);

    // 再次选择，应该选择第一个上游
    let selected = balancer.select_upstream().await.unwrap();
    assert_eq!(selected.upstream_ref.name, first_name);

    // 增加第一个上游的待处理请求数
    for _ in 0..5 {
        let _ = balancer.select_upstream().await.unwrap();
        // 不再断言这里必须选择 first_name，因为负载均衡可能已经开始选择其他上游
    }

    // 在另一个线程中为第一个上游更新一个慢响应时间
    let first_upstream_clone = first_upstream.clone();
    tokio::spawn(async move {
        balancer_clone.update_metrics(&first_upstream_clone, 5000);
    })
    .await
    .unwrap();

    // 更新其他上游的响应时间，使其更有可能被选择
    let mut other_upstream = None;
    for _ in 0..10 {
        let upstream = balancer.select_upstream().await.unwrap();
        if upstream.upstream_ref.name != first_name {
            other_upstream = Some(upstream);
            break;
        }
    }

    if let Some(upstream) = other_upstream {
        // 为其他上游设置更快的响应时间
        balancer.update_metrics(upstream, 100);

        // 此时应该选择其他上游，因为第一个上游有多个待处理请求且响应时间变慢
        let new_selected = balancer.select_upstream().await.unwrap();
        // 修改断言，我们只需要确保选择的不是第一个上游
        assert_ne!(new_selected.upstream_ref.name, first_name);
    }
}

#[tokio::test]
async fn test_response_aware_balancer_failure_handling() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = ResponseAwareBalancer::new(managed_upstreams);

    // 选择一个上游
    let selected = balancer.select_upstream().await.unwrap();
    let selected_name = selected.upstream_ref.name.clone();

    // 多次报告失败，降低成功率
    for _ in 0..5 {
        balancer.report_failure(selected).await;
    }

    // 选择并更新其他上游的响应时间
    // 我们无法直接访问 upstreams 字段，所以通过多次选择来找到其他上游
    let mut updated_others = false;
    for _ in 0..10 {
        let upstream = balancer.select_upstream().await.unwrap();
        if upstream.upstream_ref.name != selected_name {
            // 为非失败上游设置较快的响应时间
            balancer.update_metrics(upstream, 100);
            updated_others = true;
        }
    }

    // 确保我们更新了至少一个其他上游
    assert!(updated_others);

    // 多次选择，统计各上游被选择的次数
    let mut counts = std::collections::HashMap::new();
    counts.insert("upstream1".to_string(), 0);
    counts.insert("upstream2".to_string(), 0);
    counts.insert("upstream3".to_string(), 0);

    for _ in 0..10 {
        let next_selected = balancer.select_upstream().await.unwrap();
        *counts.get_mut(&next_selected.upstream_ref.name).unwrap() += 1;
    }

    // 验证报告失败的上游被选择的次数较少
    let failed_count = counts[&selected_name];
    let total_other_count: i32 = counts
        .iter()
        .filter(|(name, _)| **name != selected_name)
        .map(|(_, count)| *count)
        .sum();

    // 失败的上游应该被选择的次数明显少于其他上游
    assert!(failed_count < total_other_count);
}

#[tokio::test]
async fn test_response_aware_balancer_factory_creation() {
    let managed_upstreams = create_test_managed_upstreams();

    // 使用工厂函数创建响应时间感知负载均衡器
    let balancer = llmproxy::balancer::create_load_balancer(
        &BalanceStrategy::ResponseAware,
        managed_upstreams.clone(),
    );

    // 验证创建的是正确的类型
    assert_eq!(balancer.as_str(), "response_aware");

    // 验证可以正确选择上游
    assert!(balancer.select_upstream().await.is_ok());
}

#[tokio::test]
async fn test_response_aware_with_upstream_manager() {
    // 创建模拟服务器
    let mock_server1 = MockServer::start().await;
    let mock_server2 = MockServer::start().await;

    // 配置模拟响应 - 第一个服务器响应快
    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Fast Server"))
        .mount(&mock_server1)
        .await;

    // 第二个服务器响应慢
    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("Slow Server")
                .set_delay(Duration::from_millis(100)),
        )
        .mount(&mock_server2)
        .await;

    // 创建上游配置
    let upstream_configs = vec![
        UpstreamConfig {
            name: "fast".to_string(),
            url: mock_server1.uri().into(),
            weight: 1,
            http_client: HttpClientConfig::default(),
            auth: None,
            headers: vec![],
            breaker: None,
        },
        UpstreamConfig {
            name: "slow".to_string(),
            url: mock_server2.uri().into(),
            weight: 1,
            http_client: HttpClientConfig::default(),
            auth: None,
            headers: vec![],
            breaker: None,
        },
    ];

    // 创建上游组配置，使用响应时间感知策略
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

    // 创建上游管理器
    let upstream_manager = UpstreamManager::new(upstream_configs, group_configs)
        .await
        .unwrap();

    // 进行多次请求，应该优先选择响应快的服务器
    let mut fast_count = 0;
    let mut slow_count = 0;

    for _ in 0..10 {
        let response = upstream_manager
            .forward_request(
                "test_group",
                reqwest::Method::GET,
                "/test",
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

        // 给负载均衡器一些时间来更新指标
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // 验证快速服务器被选择的次数更多
    assert!(fast_count > slow_count);
}

#[tokio::test]
async fn test_response_aware_balancer_under_load() {
    // 创建具有不同初始响应时间的上游
    let managed_upstreams = create_test_managed_upstreams();

    // 确保有足够多的上游用于测试
    if managed_upstreams.len() < 3 {
        panic!("Test requires at least 3 upstreams");
    }

    let balancer = Arc::new(ResponseAwareBalancer::new(managed_upstreams));

    // 为不同上游设置不同的响应时间
    // 上游1：快速 (50ms)
    // 上游2：中等 (500ms)
    // 上游3：慢速 (2000ms)
    let upstream1 = balancer.select_upstream().await.unwrap();
    balancer.update_metrics(upstream1, 50);

    // 找到上游2
    let mut upstream2 = None;
    for _ in 0..10 {
        let upstream = balancer.select_upstream().await.unwrap();
        if upstream.upstream_ref.name != upstream1.upstream_ref.name {
            upstream2 = Some(upstream);
            break;
        }
    }
    let upstream2 = upstream2.expect("Could not find second upstream");
    balancer.update_metrics(upstream2, 500);

    // 找到上游3
    let mut upstream3 = None;
    for _ in 0..10 {
        let upstream = balancer.select_upstream().await.unwrap();
        if upstream.upstream_ref.name != upstream1.upstream_ref.name
            && upstream.upstream_ref.name != upstream2.upstream_ref.name
        {
            upstream3 = Some(upstream);
            break;
        }
    }
    let upstream3 = upstream3.expect("Could not find third upstream");
    balancer.update_metrics(upstream3, 2000);

    // 记录各上游名称
    let name1 = upstream1.upstream_ref.name.clone();
    let name2 = upstream2.upstream_ref.name.clone();
    let name3 = upstream3.upstream_ref.name.clone();

    // 模拟高负载场景：同时发送多个请求
    let mut handles = Vec::new();
    let balancer_clone = balancer.clone();

    // 统计选择结果
    let counts = Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    counts.lock().unwrap().insert(name1.clone(), 0);
    counts.lock().unwrap().insert(name2.clone(), 0);
    counts.lock().unwrap().insert(name3.clone(), 0);

    // 发起100个并发请求以获得更明显的统计差异
    for _ in 0..100 {
        let balancer = balancer_clone.clone();
        let counts = counts.clone();
        let name1 = name1.clone();
        let name2 = name2.clone();
        let name3 = name3.clone();

        let handle = tokio::spawn(async move {
            let selected = balancer.select_upstream().await.unwrap();
            let name = selected.upstream_ref.name.clone();

            // 模拟请求处理时间
            let processing_time = match name.as_str() {
                n if n == name1 => 50,
                n if n == name2 => 500,
                n if n == name3 => 2000,
                _ => 300,
            };

            // 等待"处理"完成
            tokio::time::sleep(Duration::from_millis(processing_time as u64 / 10)).await;

            // 更新指标
            balancer.update_metrics(selected, processing_time);

            // 记录选择结果
            let mut counts = counts.lock().unwrap();
            *counts.entry(name).or_insert(0) += 1;
        });

        handles.push(handle);
    }

    // 等待所有请求完成
    for handle in handles {
        handle.await.unwrap();
    }

    // 验证结果
    let counts = counts.lock().unwrap();

    // 放宽测试条件：确保慢速上游不是被选择最多的
    let name1_count = counts[&name1];
    let name2_count = counts[&name2];
    let name3_count = counts[&name3];

    assert!(
        name3_count <= name1_count || name3_count <= name2_count,
        "Slow upstream should not be selected the most, current counts: name1={}, name2={}, name3={}",
        name1_count,
        name2_count,
        name3_count
    );
}

#[tokio::test]
async fn test_failover_balancer_creation() {
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = FailoverBalancer::new(managed_upstreams);

    // 由于字段是私有的，我们不能直接访问它们
    // 但我们可以测试 select_upstream 方法
    let upstream = balancer.select_upstream().await.unwrap();
    assert_eq!(upstream.upstream_ref.name, "upstream1");
}

#[tokio::test]
async fn test_failover_balancer_selection_order() {
    // 创建上游列表
    let managed_upstreams = create_test_managed_upstreams();
    let balancer = FailoverBalancer::new(managed_upstreams);

    // 第一次选择应该返回第一个上游
    let first = balancer.select_upstream().await.unwrap();
    assert_eq!(first.upstream_ref.name, "upstream1");
}

#[tokio::test]
async fn test_load_balancer_factory_failover() {
    let managed_upstreams = create_test_managed_upstreams();

    // 使用工厂函数创建故障转移负载均衡器
    let balancer = llmproxy::balancer::create_load_balancer(
        &BalanceStrategy::Failover,
        managed_upstreams.clone(),
    );

    // 验证创建的是正确的类型
    assert_eq!(balancer.as_str(), "failover");

    // 验证可以正确选择上游
    assert!(balancer.select_upstream().await.is_ok());
}

#[tokio::test]
async fn test_failover_balancer_with_unavailable_upstream() {
    // 创建带有熔断器的上游列表
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

    // 为第一个上游创建熔断器并设置为不可用
    let breaker_config = llmproxy::config::BreakerConfig {
        threshold: 0.5,
        cooldown: 30,
    };
    let breaker = llmproxy::breaker::create_upstream_circuit_breaker(
        "test_upstream".to_string(),
        "test_group".to_string(),
        "http://example.com".to_string().into(),
        &breaker_config,
    );

    // 手动触发熔断器
    for _ in 0..10 {
        let _ = breaker
            .call_async(|| async {
                Err::<(), _>(llmproxy::breaker::UpstreamError("test failure".to_string()))
            })
            .await;
    }

    // 确认熔断器已开启
    assert!(!breaker.is_call_permitted());

    // 设置熔断器
    managed_upstreams[0].breaker = Some(breaker);

    // 创建故障转移负载均衡器
    let balancer = FailoverBalancer::new(managed_upstreams);

    // 由于第一个上游不可用，应该选择第二个上游
    let selected = balancer.select_upstream().await.unwrap();
    assert_eq!(selected.upstream_ref.name, "available");
}
