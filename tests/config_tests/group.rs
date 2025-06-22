// tests/config/group.rs

// This module contains tests for the UpstreamGroupConfig struct.

use super::common::TestConfigBuilder;
use llmproxy::config::{HttpClientConfig, HttpClientTimeoutConfig, ProxyConfig};
use validator::Validate;

#[test]
fn test_config_with_proxy() {
    let config = TestConfigBuilder::new()
        .map_config(|c| {
            // Create a proxy config and apply it to the default group
            let http_client_config = HttpClientConfig {
                timeout: HttpClientTimeoutConfig::default(),
                keepalive: 60,
                retry: None,
                proxy: Some(ProxyConfig {
                    url: "http://proxy.example.com:8080".to_string(),
                }),
                stream_mode: false,
            };
            c.upstream_groups[0].http_client = http_client_config;
        })
        .build();

    // Validate the modified config
    assert!(config.validate().is_ok());

    // Test serialization and deserialization
    let (_dir, file_path) = super::common::create_temp_config_file(&config);
    let deserialized_config = llmproxy::config::Config::from_file(file_path).unwrap();

    // Verify proxy config is retained
    let group = &deserialized_config.upstream_groups[0];
    assert!(group.http_client.proxy.is_some());
    assert_eq!(
        group.http_client.proxy.as_ref().unwrap().url,
        "http://proxy.example.com:8080"
    );
}
