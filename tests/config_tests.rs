use llmproxy::config::{
    AdminConfig, AuthConfig, AuthType, BalanceConfig, BalanceStrategy, Config, ForwardConfig,
    HttpClientConfig, HttpClientTimeoutConfig, HttpServerConfig, ProxyConfig, RateLimitConfig,
    TimeoutConfig, UpstreamConfig, UpstreamGroupConfig, UpstreamRef,
};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;
use validator::Validate;

// 创建有效的测试配置
fn create_valid_test_config() -> Config {
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
        upstream_group: "test_group".to_string(),
        ratelimit: Some(RateLimitConfig {
            per_second: 100,
            burst: 200,
        }),
        timeout: Some(TimeoutConfig { connect: 5 }),
    };

    Config {
        http_server: Some(HttpServerConfig {
            forwards: vec![forward_config],
            admin: AdminConfig {
                port: 9000,
                address: "127.0.0.1".to_string(),
                timeout: Some(TimeoutConfig { connect: 5 }),
            },
        }),
        upstreams: vec![upstream_config],
        upstream_groups: vec![group_config],
    }
}

#[test]
fn test_config_validation_valid() {
    let config = create_valid_test_config();
    let result = config.validate();
    assert!(result.is_ok());
}

#[test]
fn test_config_validation_duplicate_names() {
    let mut config = create_valid_test_config();

    // 添加重复的上游名称
    let duplicate_upstream = UpstreamConfig {
        name: "test_upstream".to_string(), // 重复的名称
        url: "http://localhost:8081".to_string().into(),
        weight: 1,
        http_client: HttpClientConfig::default(),
        auth: None,
        headers: vec![],
        breaker: None,
    };
    config.upstreams.push(duplicate_upstream);

    let result = config.validate();
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Duplicate"));
    } else {
        panic!("Expected Config error for duplicate name");
    }
}

#[test]
fn test_config_validation_invalid_url() {
    let mut config = create_valid_test_config();

    // 设置无效的URL
    config.upstreams[0].url = "invalid-url".to_string().into();

    // 现在我们期望 config.validate() 能够捕获 URL 错误
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
    let mut config = create_valid_test_config();

    // 先创建一个熔断器配置并赋值给 breaker 字段
    use llmproxy::config::BreakerConfig;
    use llmproxy::r#const::breaker_limits;

    config.upstreams[0].breaker = Some(BreakerConfig {
        threshold: breaker_limits::MAX_THRESHOLD + 1.0, // 超出有效范围
        cooldown: breaker_limits::DEFAULT_COOLDOWN,
    });

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
    let mut config = create_valid_test_config();

    // 设置无效的认证配置
    config.upstreams[0].auth = Some(AuthConfig {
        r#type: AuthType::Bearer,
        token: None, // Bearer认证需要token
        username: None,
        password: None,
    });

    let result = config.validate();
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("token"));
    } else {
        panic!("Expected Config error for invalid auth config");
    }
}

#[test]
fn test_config_validation_missing_upstream_reference() {
    let mut config = create_valid_test_config();

    // 添加引用不存在的上游的组
    let invalid_group = UpstreamGroupConfig {
        name: "invalid_group".to_string(),
        upstreams: vec![UpstreamRef {
            name: "non_existent_upstream".to_string(),
            weight: 1,
        }],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: HttpClientConfig::default(),
    };
    config.upstream_groups.push(invalid_group);

    let result = config.validate();
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("non_existent_upstream"));
    } else {
        panic!("Expected Config error for missing upstream reference");
    }
}

#[test]
fn test_config_from_file() {
    let config = create_valid_test_config();

    // 创建临时目录
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_config.yaml");

    // 序列化配置并写入文件
    let yaml = serde_yaml::to_string(&config).unwrap();
    let mut file = File::create(&file_path).unwrap();
    file.write_all(yaml.as_bytes()).unwrap();

    // 从文件加载配置
    let loaded_config = Config::from_file(&file_path).unwrap();

    // 验证配置
    assert_eq!(loaded_config.upstreams.len(), config.upstreams.len());
    assert_eq!(
        loaded_config.upstream_groups.len(),
        config.upstream_groups.len()
    );
    assert_eq!(
        loaded_config.http_server.as_ref().unwrap().forwards.len(),
        config.http_server.as_ref().unwrap().forwards.len()
    );

    // 验证上游名称
    assert_eq!(loaded_config.upstreams[0].name, "test_upstream");

    // 验证上游组名称
    assert_eq!(loaded_config.upstream_groups[0].name, "test_group");

    // 验证转发服务名称
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
    // 创建临时目录
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("invalid_config.yaml");

    // 写入无效的YAML内容
    let mut file = File::create(&file_path).unwrap();
    file.write_all(b"invalid: yaml: content:").unwrap();

    let result = Config::from_file(&file_path);
    assert!(result.is_err());
}

#[test]
fn test_load_config_from_file_with_http_server() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test_config.yaml");

    let config = create_valid_test_config();
    let yaml = serde_yaml::to_string(&config).unwrap();
    let mut file = File::create(&file_path).unwrap();
    file.write_all(yaml.as_bytes()).unwrap();

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
fn test_config_with_proxy() {
    // 创建一个带有 proxy 配置的 HttpClientConfig
    let http_client_config = HttpClientConfig {
        timeout: HttpClientTimeoutConfig::default(),
        keepalive: 60,
        retry: None,
        proxy: Some(ProxyConfig {
            url: "http://proxy.example.com:8080".to_string(),
        }),
        stream_mode: false,
    };

    // 创建一个使用该 HttpClientConfig 的 UpstreamGroupConfig
    let group_config = UpstreamGroupConfig {
        name: "test_group".to_string(),
        upstreams: vec![UpstreamRef {
            name: "test_upstream".to_string(),
            weight: 1,
        }],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: http_client_config,
    };

    // 创建一个包含该 UpstreamGroupConfig 的 Config
    let config = Config {
        http_server: None,
        upstreams: vec![UpstreamConfig {
            name: "test_upstream".to_string(),
            url: "http://localhost:8080".to_string().into(),
            weight: 1,
            http_client: HttpClientConfig::default(),
            auth: None,
            headers: vec![],
            breaker: None,
        }],
        upstream_groups: vec![group_config],
    };

    // 验证配置是否有效
    assert!(config.validate().is_ok());

    // 测试序列化和反序列化
    let yaml = serde_yaml::to_string(&config).unwrap();
    let deserialized_config: Config = serde_yaml::from_str(&yaml).unwrap();

    // 验证 proxy 配置是否正确保留
    assert!(deserialized_config.upstream_groups[0]
        .http_client
        .proxy
        .is_some());
    assert_eq!(
        deserialized_config.upstream_groups[0]
            .http_client
            .proxy
            .as_ref()
            .unwrap()
            .url,
        "http://proxy.example.com:8080"
    );
}

#[test]
fn test_config_with_none_options() {
    // 创建一个 ForwardConfig，其中 ratelimit 和 timeout 明确设置为 None
    let forward_config = ForwardConfig {
        name: "test_forward".to_string(),
        port: 3000,
        address: "127.0.0.1".to_string(),
        upstream_group: "test_group".to_string(),
        ratelimit: None,
        timeout: None,
    };

    // 创建一个包含该 ForwardConfig 的 Config
    let config = Config {
        http_server: Some(HttpServerConfig {
            forwards: vec![forward_config],
            admin: AdminConfig {
                port: 9000,
                address: "127.0.0.1".to_string(),
                timeout: None,
            },
        }),
        upstreams: vec![UpstreamConfig {
            name: "test_upstream".to_string(),
            url: "http://localhost:8080".to_string().into(),
            weight: 1,
            http_client: HttpClientConfig::default(),
            auth: None,
            headers: vec![],
            breaker: None,
        }],
        upstream_groups: vec![UpstreamGroupConfig {
            name: "test_group".to_string(),
            upstreams: vec![UpstreamRef {
                name: "test_upstream".to_string(),
                weight: 1,
            }],
            balance: BalanceConfig {
                strategy: BalanceStrategy::RoundRobin,
            },
            http_client: HttpClientConfig::default(),
        }],
    };

    // 验证配置是否有效
    assert!(config.validate().is_ok());

    // 测试序列化和反序列化
    let yaml = serde_yaml::to_string(&config).unwrap();
    let deserialized_config: Config = serde_yaml::from_str(&yaml).unwrap();

    // 验证 None 值是否正确保留
    assert!(deserialized_config.http_server.is_some());
    let http_server = deserialized_config.http_server.unwrap();
    assert!(http_server.forwards[0].ratelimit.is_none());
    assert!(http_server.forwards[0].timeout.is_none());
    assert!(http_server.admin.timeout.is_none());
}

#[test]
fn test_config_without_http_server() {
    // 创建一个 http_server 为 None 的 Config
    let config = Config {
        http_server: None,
        upstreams: vec![UpstreamConfig {
            name: "test_upstream".to_string(),
            url: "http://localhost:8080".to_string().into(),
            weight: 1,
            http_client: HttpClientConfig::default(),
            auth: None,
            headers: vec![],
            breaker: None,
        }],
        upstream_groups: vec![UpstreamGroupConfig {
            name: "test_group".to_string(),
            upstreams: vec![UpstreamRef {
                name: "test_upstream".to_string(),
                weight: 1,
            }],
            balance: BalanceConfig {
                strategy: BalanceStrategy::RoundRobin,
            },
            http_client: HttpClientConfig::default(),
        }],
    };

    // 验证配置是否有效
    assert!(config.validate().is_ok());

    // 测试序列化和反序列化
    let yaml = serde_yaml::to_string(&config).unwrap();
    let deserialized_config: Config = serde_yaml::from_str(&yaml).unwrap();

    // 验证 http_server 是否仍然为 None
    assert!(deserialized_config.http_server.is_none());
}
