// tests/config/validation.rs

// This module contains tests for cross-cutting config validation logic.

use super::common::TestConfigBuilder;
use llmproxy::config::{
    http_server::RoutingRule, BalanceConfig, BalanceStrategy, HttpClientConfig, UpstreamConfig,
    UpstreamGroupConfig, UpstreamRef,
};
use validator::Validate;

#[test]
fn test_config_validation_valid() {
    let config = TestConfigBuilder::new().build();
    let result = config.validate();
    assert!(result.is_ok());
}

#[test]
fn test_config_validation_duplicate_names() {
    let duplicate_upstream = UpstreamConfig {
        name: "test_upstream".to_string(), // Duplicate name
        url: "http://localhost:8081".to_string().into(),
        weight: 1,
        http_client: HttpClientConfig::default(),
        auth: None,
        headers: vec![],
        breaker: None,
    };

    let config = TestConfigBuilder::new()
        .with_upstream(duplicate_upstream)
        .build();

    let result = config.validate();
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Duplicate"));
    } else {
        panic!("Expected Config error for duplicate name");
    }
}

#[test]
fn test_config_validation_duplicate_upstreams_in_group() {
    let duplicate_group = UpstreamGroupConfig {
        name: "duplicate_upstream_group".to_string(),
        upstreams: vec![
            UpstreamRef {
                name: "test_upstream".to_string(),
                weight: 1,
            },
            UpstreamRef {
                name: "test_upstream".to_string(), // 重复的上游引用
                weight: 2,
            },
        ],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: HttpClientConfig::default(),
    };

    let config = TestConfigBuilder::new().with_group(duplicate_group).build();

    let result = config.validate();
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Duplicate upstream"));
    } else {
        panic!("Expected Config error for duplicate upstream references");
    }
}

#[test]
fn test_config_validation_missing_upstream_reference() {
    let invalid_group = UpstreamGroupConfig {
        name: "invalid_group".to_string(),
        upstreams: vec![llmproxy::config::UpstreamRef {
            name: "non_existent_upstream".to_string(),
            weight: 1,
        }],
        balance: llmproxy::config::BalanceConfig {
            strategy: llmproxy::config::BalanceStrategy::RoundRobin,
        },
        http_client: HttpClientConfig::default(),
    };

    let config = TestConfigBuilder::new().with_group(invalid_group).build();

    let result = config.validate();
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("non_existent_upstream"));
    } else {
        panic!("Expected Config error for missing upstream reference");
    }
}

#[test]
fn test_config_validation_duplicate_routing_paths() {
    let duplicate_routing_rules = vec![
        RoutingRule {
            path: "/api/v1/chat".to_string(),
            target_group: "test_group".to_string(),
        },
        RoutingRule {
            path: "/api/v1/chat".to_string(), // 重复的路径
            target_group: "another_group".to_string(),
        },
    ];

    let config = TestConfigBuilder::new()
        .map_config(|c| {
            if let Some(http_server) = c.http_server.as_mut() {
                if !http_server.forwards.is_empty() {
                    http_server.forwards[0].routing = Some(duplicate_routing_rules);
                }
            }
        })
        .build();

    let result = config.validate();
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Duplicate path pattern"));
    } else {
        panic!("Expected Config error for duplicate routing paths");
    }
}
