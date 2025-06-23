// tests/balancer/integration.rs

// This module contains integration tests for balancers with UpstreamManager.

use llmproxy::{
    config::{
        BalanceConfig, BalanceStrategy, HttpClientConfig, UpstreamConfig, UpstreamGroupConfig,
        UpstreamRef,
    },
    upstream::UpstreamManager,
};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
async fn test_load_balancer_with_upstream_manager() {
    let mock_server1 = MockServer::start().await;
    let mock_server2 = MockServer::start().await;

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

    let upstream_configs = vec![
        UpstreamConfig {
            name: "upstream1".to_string(),
            url: format!("{}/test", mock_server1.uri()).into(),
            weight: 1,
            http_client: HttpClientConfig::default(),
            auth: None,
            headers: vec![],
            breaker: None,
        },
        UpstreamConfig {
            name: "upstream2".to_string(),
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

    let upstream_manager = UpstreamManager::new(upstream_configs, group_configs)
        .await
        .unwrap();

    let response1 = upstream_manager
        .forward_request(
            "test_group",
            &reqwest::Method::GET,
            reqwest::header::HeaderMap::new(),
            None,
        )
        .await;
    assert!(response1.is_ok());

    let response2 = upstream_manager
        .forward_request(
            "test_group",
            &reqwest::Method::GET,
            reqwest::header::HeaderMap::new(),
            None,
        )
        .await;
    assert!(response2.is_ok());

    let body1 = response1.unwrap().text().await.unwrap();
    let body2 = response2.unwrap().text().await.unwrap();
    assert_ne!(body1, body2);
}

#[tokio::test]
async fn test_load_balancer_with_unavailable_upstream() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Server OK"))
        .mount(&mock_server)
        .await;

    let upstream_configs = vec![
        UpstreamConfig {
            name: "available".to_string(),
            url: format!("{}/test", mock_server.uri()).into(),
            weight: 1,
            http_client: HttpClientConfig::default(),
            auth: None,
            headers: vec![],
            breaker: None,
        },
        UpstreamConfig {
            name: "unavailable".to_string(),
            url: "http://localhost:1".to_string().into(), // Unavailable
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

    let upstream_manager = UpstreamManager::new(upstream_configs, group_configs)
        .await
        .unwrap();

    let response = upstream_manager
        .forward_request(
            "test_group",
            &reqwest::Method::GET,
            reqwest::header::HeaderMap::new(),
            Some("/test".to_string().into()),
        )
        .await;

    assert!(response.is_ok());
    let body = response.unwrap().text().await.unwrap();
    assert_eq!(body, "Server OK");
}
