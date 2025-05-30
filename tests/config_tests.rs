use llmproxy::{
    config::{
        AuthConfig, AuthType, BalanceConfig, BalanceStrategy, BreakerConfig, Config, ForwardConfig,
        HeaderOpType, HeaderOperation, HttpClientConfig, HttpClientTimeoutConfig, ProxyConfig,
        RateLimitConfig, RetryConfig, TimeoutConfig, UpstreamConfig, UpstreamGroupConfig,
        UpstreamRef,
    },
    error::AppError,
};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;
use uuid::Uuid;

// 创建有效的测试配置
fn create_valid_test_config() -> Config {
    let upstream_config = UpstreamConfig {
        name: "test_upstream".to_string(),
        url: "http://localhost:8080".to_string(),
        id: Uuid::new_v4().to_string(),
        auth: Some(AuthConfig {
            r#type: AuthType::Bearer,
            token: Some("test_token".to_string()),
            username: None,
            password: None,
        }),
        headers: vec![HeaderOperation {
            op: HeaderOpType::Insert,
            key: "X-Test-Header".to_string(),
            value: Some("test-value".to_string()),
        }],
        breaker: Some(BreakerConfig {
            threshold: 0.5,
            cooldown: 30,
        }),
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
        http_client: HttpClientConfig {
            agent: "Test-Agent".to_string(),
            keepalive: 30,
            timeout: HttpClientTimeoutConfig {
                connect: 5,
                request: 30,
                idle: 60,
            },
            retry: RetryConfig {
                enabled: false,
                attempts: 1,
                initial: 500,
            },
            proxy: ProxyConfig {
                enabled: false,
                url: "".to_string(),
            },
            stream_mode: false,
        },
    };

    let forward_config = ForwardConfig {
        name: "test_forward".to_string(),
        port: 3000,
        address: "127.0.0.1".to_string(),
        upstream_group: "test_group".to_string(),
        ratelimit: RateLimitConfig {
            enabled: false,
            per_second: 100,
            burst: 200,
        },
        timeout: TimeoutConfig { connect: 5 },
    };

    Config {
        http_server: llmproxy::config::HttpServerConfig {
            forwards: vec![forward_config],
            admin: llmproxy::config::AdminConfig {
                port: 9000,
                address: "127.0.0.1".to_string(),
                timeout: TimeoutConfig { connect: 5 },
            },
        },
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
        url: "http://localhost:8081".to_string(),
        id: Uuid::new_v4().to_string(),
        auth: None,
        headers: vec![],
        breaker: None,
    };
    config.upstreams.push(duplicate_upstream);

    let result = config.validate();
    assert!(result.is_err());
    if let Err(AppError::Config(msg)) = result {
        assert!(msg.contains("duplicated"));
    } else {
        panic!("Expected Config error for duplicate name");
    }
}

#[test]
fn test_config_validation_invalid_url() {
    let mut config = create_valid_test_config();

    // 设置无效的URL
    config.upstreams[0].url = "invalid-url".to_string();

    let result = config.validate();
    assert!(result.is_err());
    if let Err(AppError::Config(msg)) = result {
        assert!(msg.contains("invalid"));
    } else {
        panic!("Expected Config error for invalid URL");
    }
}

#[test]
fn test_config_validation_invalid_breaker_config() {
    let mut config = create_valid_test_config();

    // 设置无效的熔断器阈值
    if let Some(breaker) = &mut config.upstreams[0].breaker {
        breaker.threshold = 2.0; // 超出有效范围
    }

    let result = config.validate();
    assert!(result.is_err());
    if let Err(AppError::Config(msg)) = result {
        assert!(msg.contains("threshold"));
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
    if let Err(AppError::Config(msg)) = result {
        assert!(msg.contains("token"));
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
    if let Err(AppError::Config(msg)) = result {
        assert!(msg.contains("non_existent_upstream"));
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
        loaded_config.http_server.forwards.len(),
        config.http_server.forwards.len()
    );

    // 验证上游名称
    assert_eq!(loaded_config.upstreams[0].name, "test_upstream");

    // 验证上游组名称
    assert_eq!(loaded_config.upstream_groups[0].name, "test_group");

    // 验证转发服务名称
    assert_eq!(loaded_config.http_server.forwards[0].name, "test_forward");
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
