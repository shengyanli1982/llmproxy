// tests/config/file.rs

// This module contains tests for config file loading and parsing.

use super::common::{create_temp_config_file, TestConfigBuilder};
use llmproxy::config::Config;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;
use validator::Validate;

#[test]
fn test_config_from_file() {
    let config = TestConfigBuilder::new().build();
    let (_dir, file_path) = create_temp_config_file(&config);

    // Load from the file
    let loaded_config = Config::from_file(&file_path).unwrap();

    // Basic validation
    assert_eq!(loaded_config.upstreams.len(), config.upstreams.len());
    assert_eq!(
        loaded_config.upstream_groups.len(),
        config.upstream_groups.len()
    );
    assert_eq!(
        loaded_config.http_server.as_ref().unwrap().forwards.len(),
        config.http_server.as_ref().unwrap().forwards.len()
    );
    assert_eq!(loaded_config.upstreams[0].name, "test_upstream");
    assert_eq!(loaded_config.upstream_groups[0].name, "test_group");
    assert_eq!(
        loaded_config.http_server.unwrap().forwards[0].name,
        "test_forward"
    );
}

#[test]
fn test_config_from_file_invalid_path() {
    let result = Config::from_file("non_existent_file.yaml");
    assert!(result.is_err());
}

#[test]
fn test_config_from_file_invalid_content() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("invalid_config.yaml");

    // Write invalid YAML content
    let mut file = File::create(&file_path).unwrap();
    file.write_all(b"invalid: yaml: content:").unwrap();

    let result = Config::from_file(&file_path);
    assert!(result.is_err());
}

#[test]
fn test_load_config_from_file_with_http_server() {
    let config = TestConfigBuilder::new().build();
    let (_dir, file_path) = create_temp_config_file(&config);

    let loaded_config = Config::from_file(file_path.to_str().unwrap()).unwrap();

    assert_eq!(
        loaded_config.http_server.as_ref().unwrap().forwards.len(),
        config.http_server.as_ref().unwrap().forwards.len()
    );
    assert_eq!(
        loaded_config.http_server.unwrap().forwards[0].name,
        "test_forward"
    );
}

#[test]
fn test_config_without_http_server() {
    let config = TestConfigBuilder::new()
        .map_config(|c| {
            c.http_server = None;
        })
        .build();

    // The config is still valid without an http_server
    assert!(config.validate().is_ok());

    let (_dir, file_path) = create_temp_config_file(&config);
    let loaded_config = Config::from_file(file_path).unwrap();

    // Verify http_server is still None after deserialization
    assert!(loaded_config.http_server.is_none());
}
