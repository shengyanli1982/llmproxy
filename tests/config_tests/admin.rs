// tests/config/admin.rs

// This module contains tests for the AdminConfig struct.
use super::common::{create_temp_config_file, TestConfigBuilder};
use validator::Validate;

#[test]
fn test_admin_with_none_options() {
    let config = TestConfigBuilder::new()
        .map_config(|c| {
            c.http_server.as_mut().unwrap().admin.timeout = None;
        })
        .build();

    assert!(config.validate().is_ok());

    let (_dir, file_path) = create_temp_config_file(&config);
    let deserialized = llmproxy::config::Config::from_file(file_path).unwrap();

    let admin = &deserialized.http_server.unwrap().admin;
    assert!(admin.timeout.is_none());
}
