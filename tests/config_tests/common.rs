// tests/config/common.rs

// This module will contain shared helper functions and builders for the config tests.

use llmproxy::config::{
    AdminConfig, BalanceConfig, BalanceStrategy, Config, ForwardConfig, HttpClientConfig,
    RateLimitConfig, TimeoutConfig, UpstreamConfig, UpstreamGroupConfig, UpstreamRef,
};

// A builder for creating `Config` instances for testing purposes.
pub struct TestConfigBuilder {
    config: Config,
}

impl TestConfigBuilder {
    /// Creates a new builder with a default, valid configuration.
    pub fn new() -> Self {
        let upstream_config = UpstreamConfig {
            name: "test_upstream".to_string(),
            url: "http://localhost:8080".to_string().into(),
            weight: 1,
            http_client: HttpClientConfig::default(),
            auth: None,
            headers: vec![],
            breaker: None,
        };

        let upstream_ref = UpstreamRef {
            name: "test_upstream".to_string(),
            weight: 1,
        };

        let group_config = UpstreamGroupConfig {
            name: "test_group".to_string(),
            upstreams: vec![upstream_ref],
            balance: BalanceConfig {
                strategy: BalanceStrategy::RoundRobin,
            },
            http_client: HttpClientConfig::default(),
        };

        let forward_config = ForwardConfig {
            name: "test_forward".to_string(),
            port: 3000,
            address: "127.0.0.1".to_string(),
            default_group: "test_group".to_string(),
            ratelimit: Some(RateLimitConfig {
                per_second: 100,
                burst: 200,
            }),
            timeout: Some(TimeoutConfig { connect: 5 }),
            routing: None,
        };

        let config = Config {
            http_server: Some(llmproxy::config::HttpServerConfig {
                forwards: vec![forward_config],
                admin: AdminConfig {
                    port: 9000,
                    address: "127.0.0.1".to_string(),
                    timeout: Some(TimeoutConfig { connect: 5 }),
                },
            }),
            upstreams: vec![upstream_config],
            upstream_groups: vec![group_config],
        };

        Self { config }
    }

    /// Consumes the builder and returns the `Config`.
    pub fn build(self) -> Config {
        self.config
    }

    /// Adds an additional upstream configuration.
    pub fn with_upstream(mut self, upstream: UpstreamConfig) -> Self {
        self.config.upstreams.push(upstream);
        self
    }

    /// Adds an additional upstream group configuration.
    pub fn with_group(mut self, group: UpstreamGroupConfig) -> Self {
        self.config.upstream_groups.push(group);
        self
    }

    /// Applies a custom modification to the configuration.
    pub fn map_config<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut Config),
    {
        f(&mut self.config);
        self
    }
}

/// Helper function to create a temporary config file.
/// Returns the TempDir (to keep it alive) and the path to the file.
pub fn create_temp_config_file(config: &Config) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test_config.yaml");
    let yaml = serde_yaml::to_string(config).unwrap();
    std::fs::write(&file_path, yaml).unwrap();
    (dir, file_path)
}

impl Default for TestConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
