// tests/config/routing.rs

// This module contains tests for the routing functionality within ForwardConfig.

use super::common::TestConfigBuilder;
use llmproxy::config::{
    http_server::RoutingRule, BalanceConfig, BalanceStrategy, UpstreamGroupConfig, UpstreamRef,
};
use validator::Validate;

#[test]
fn test_config_with_routing() {
    let api_group_config = UpstreamGroupConfig {
        name: "api_group".to_string(),
        upstreams: vec![UpstreamRef {
            name: "test_upstream".to_string(),
            weight: 1,
        }],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: Default::default(),
    };

    let routing_rules = vec![RoutingRule {
        path: "/api".to_string(),
        target_group: "api_group".to_string(),
    }];

    let config = TestConfigBuilder::new()
        .with_group(api_group_config)
        .map_config(|c| {
            c.http_server.as_mut().unwrap().forwards[0].routing = Some(routing_rules);
        })
        .build();

    let result = config.validate();
    assert!(result.is_ok());
}

#[test]
fn test_config_validation_invalid_routing_target_group() {
    let routing_rules = vec![RoutingRule {
        path: "/api".to_string(),
        target_group: "non_existent_group".to_string(),
    }];

    let config = TestConfigBuilder::new()
        .map_config(|c| {
            c.http_server.as_mut().unwrap().forwards[0].routing = Some(routing_rules);
        })
        .build();

    let result = config.validate();
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("non_existent_group"));
        assert!(e.to_string().contains("unknown upstream group"));
    } else {
        panic!("Expected Config error for unknown upstream group reference");
    }
}

#[test]
fn test_config_with_various_routing_paths() {
    let static_group = UpstreamGroupConfig {
        name: "static_group".to_string(),
        upstreams: vec![UpstreamRef {
            name: "test_upstream".to_string(),
            weight: 1,
        }],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: Default::default(),
    };
    let param_group = UpstreamGroupConfig {
        name: "param_group".to_string(),
        upstreams: vec![UpstreamRef {
            name: "test_upstream".to_string(),
            weight: 1,
        }],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: Default::default(),
    };
    let regex_group = UpstreamGroupConfig {
        name: "regex_group".to_string(),
        upstreams: vec![UpstreamRef {
            name: "test_upstream".to_string(),
            weight: 1,
        }],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: Default::default(),
    };
    let wildcard_group = UpstreamGroupConfig {
        name: "wildcard_group".to_string(),
        upstreams: vec![UpstreamRef {
            name: "test_upstream".to_string(),
            weight: 1,
        }],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: Default::default(),
    };

    let routing_rules = vec![
        RoutingRule {
            path: "/api/users/admin".to_string(),
            target_group: "static_group".to_string(),
        },
        RoutingRule {
            path: "/api/users/:id".to_string(),
            target_group: "param_group".to_string(),
        },
        RoutingRule {
            path: "/api/items/{id:[0-9]+}".to_string(),
            target_group: "regex_group".to_string(),
        },
        RoutingRule {
            path: "/api/products/{code:[A-Z][A-Z][A-Z][0-9][0-9][0-9]}".to_string(),
            target_group: "regex_group".to_string(),
        },
        RoutingRule {
            path: "/api/*/docs".to_string(),
            target_group: "wildcard_group".to_string(),
        },
        RoutingRule {
            path: "/files/*".to_string(),
            target_group: "wildcard_group".to_string(),
        },
        RoutingRule {
            path: "/api/:version/users/{id:[0-9]+}/profile".to_string(),
            target_group: "regex_group".to_string(),
        },
    ];

    let config = TestConfigBuilder::new()
        .with_group(static_group)
        .with_group(param_group)
        .with_group(regex_group)
        .with_group(wildcard_group)
        .map_config(|c| {
            c.http_server.as_mut().unwrap().forwards[0].routing = Some(routing_rules);
            // The default group in the builder is "test_group", rename it to avoid conflict if we want to test default routing
            c.upstream_groups[0].name = "default_group".to_string();
            c.http_server.as_mut().unwrap().forwards[0].default_group = "default_group".to_string();
        })
        .build();

    let result = config.validate();
    assert!(result.is_ok());
}
