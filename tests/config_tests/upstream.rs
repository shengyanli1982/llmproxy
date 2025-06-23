// tests/config/upstream.rs

// This module contains tests for the UpstreamConfig struct.

use super::common::TestConfigBuilder;
use llmproxy::config::{AuthConfig, AuthType, BreakerConfig};
use llmproxy::r#const::breaker_limits;
use validator::Validate;

#[test]
fn test_config_validation_invalid_url() {
    let config = TestConfigBuilder::new()
        .map_config(|c| {
            c.upstreams[0].url = "invalid-url".to_string().into();
        })
        .build();

    let result = config.validate();
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("URL"));
    } else {
        panic!("Expected Config error for invalid URL");
    }
}

#[test]
fn test_config_validation_invalid_breaker_config() {
    let config = TestConfigBuilder::new()
        .map_config(|c| {
            c.upstreams[0].breaker = Some(BreakerConfig {
                threshold: breaker_limits::MAX_THRESHOLD + 1.0, // Out of valid range
                cooldown: breaker_limits::DEFAULT_COOLDOWN,
            });
        })
        .build();

    let result = config.validate();
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("threshold"));
    } else {
        panic!("Expected Config error for invalid breaker threshold");
    }
}

#[test]
fn test_config_validation_invalid_auth_config() {
    let config = TestConfigBuilder::new()
        .map_config(|c| {
            c.upstreams[0].auth = Some(AuthConfig {
                r#type: AuthType::Bearer,
                token: None, // Bearer auth requires a token
                username: None,
                password: None,
            });
        })
        .build();

    let result = config.validate();
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("token"));
    } else {
        panic!("Expected Config error for invalid auth config");
    }
}
