use llmproxy::{
    balancer::{
        LoadBalancer, ManagedUpstream, RandomBalancer, RoundRobinBalancer,
        WeightedRoundRobinBalancer,
    },
    config::{BalanceConfig, BalanceStrategy, UpstreamConfig, UpstreamGroupConfig, UpstreamRef},
    upstream::UpstreamManager,
};
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
            url: mock_server1.uri(),
            auth: None,
            headers: vec![],
            breaker: None,
        },
        UpstreamConfig {
            name: "upstream2".to_string(),
            url: mock_server2.uri(),
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
            url: mock_server.uri(),
            auth: None,
            headers: vec![],
            breaker: None,
        },
        UpstreamConfig {
            name: "unavailable".to_string(),
            url: "http://localhost:1".to_string(), // 不可用的上游
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
