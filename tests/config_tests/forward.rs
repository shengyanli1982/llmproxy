// tests/config/forward.rs

// This module contains tests for the ForwardConfig struct.
use super::common::{create_temp_config_file, TestConfigBuilder};
use validator::Validate;

#[test]
fn test_forward_with_none_options() {
    let config = TestConfigBuilder::new()
        .map_config(|c| {
            let forward = &mut c.http_server.as_mut().unwrap().forwards[0];
            forward.ratelimit = None;
            forward.timeout = None;
        })
        .build();

    assert!(config.validate().is_ok());

    let (_dir, file_path) = create_temp_config_file(&config);
    let deserialized = llmproxy::config::Config::from_file(file_path).unwrap();

    let forward = &deserialized.http_server.unwrap().forwards[0];
    assert!(forward.ratelimit.is_none());
    assert!(forward.timeout.is_none());
}
