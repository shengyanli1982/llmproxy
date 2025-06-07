use crate::apis::v1::common::create_test_config;
use llmproxy::{
    apis::v1::validation::ConfigValidation,
    config::{
        AuthConfig, AuthType, BalanceConfig, BalanceStrategy, ForwardConfig, RateLimitConfig,
        TimeoutConfig, UpstreamConfig, UpstreamGroupConfig, UpstreamRef,
    },
};
use tokio::test;
use uuid::Uuid;

/// 测试上游配置验证
#[test]
async fn test_upstream_validation() {
    let config = create_test_config();

    // 测试有效的URL
    let valid_upstream = UpstreamConfig {
        name: "valid_upstream".to_string(),
        url: "http://localhost:8080".to_string(),
        id: Uuid::new_v4().to_string(),
        auth: None,
        headers: vec![],
        breaker: None,
    };

    // 验证有效的上游配置
    let result = config.validate_upstream_config(&valid_upstream);
    assert!(result.is_ok());

    // 测试无效的URL
    let invalid_url_upstream = UpstreamConfig {
        name: "invalid_url".to_string(),
        url: "invalid-url".to_string(), // 无效URL
        id: Uuid::new_v4().to_string(),
        auth: None,
        headers: vec![],
        breaker: None,
    };

    // 验证无效的URL
    let result = config.validate_upstream_config(&invalid_url_upstream);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("invalid URL"));

    // 测试无效的Bearer认证配置（缺少token）
    let invalid_auth_upstream = UpstreamConfig {
        name: "invalid_auth".to_string(),
        url: "http://localhost:8080".to_string(),
        id: Uuid::new_v4().to_string(),
        auth: Some(AuthConfig {
            r#type: AuthType::Bearer,
            token: None, // 缺少token
            username: None,
            password: None,
        }),
        headers: vec![],
        breaker: None,
    };

    // 验证无效的认证配置
    let result = config.validate_upstream_config(&invalid_auth_upstream);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("token"));

    // 测试无效的Basic认证配置（缺少用户名或密码）
    let invalid_basic_auth_upstream = UpstreamConfig {
        name: "invalid_basic_auth".to_string(),
        url: "http://localhost:8080".to_string(),
        id: Uuid::new_v4().to_string(),
        auth: Some(AuthConfig {
            r#type: AuthType::Basic,
            token: None,
            username: Some("user".to_string()),
            password: None, // 缺少密码
        }),
        headers: vec![],
        breaker: None,
    };

    // 验证无效的Basic认证配置
    let result = config.validate_upstream_config(&invalid_basic_auth_upstream);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("password"));
}

/// 测试上游组配置验证
#[test]
async fn test_upstream_group_validation() {
    let config = create_test_config();

    // 测试有效的上游组配置
    let valid_group = UpstreamGroupConfig {
        name: "valid_group".to_string(),
        upstreams: vec![UpstreamRef {
            name: "test_upstream".to_string(), // 已存在的上游
            weight: 1,
        }],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: Default::default(),
    };

    // 验证有效的上游组配置
    let result = config.validate_upstream_group_config(&valid_group);
    assert!(result.is_ok());

    // 测试引用不存在上游的上游组
    let invalid_ref_group = UpstreamGroupConfig {
        name: "invalid_ref".to_string(),
        upstreams: vec![UpstreamRef {
            name: "nonexistent_upstream".to_string(), // 不存在的上游
            weight: 1,
        }],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: Default::default(),
    };

    // 验证引用不存在上游的上游组
    let result = config.validate_upstream_group_config(&invalid_ref_group);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));

    // 测试空上游列表的上游组
    let empty_upstreams_group = UpstreamGroupConfig {
        name: "empty_upstreams".to_string(),
        upstreams: vec![], // 空上游列表
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: Default::default(),
    };

    // 验证空上游列表的上游组
    let result = config.validate_upstream_group_config(&empty_upstreams_group);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("empty"));
}

/// 测试转发规则配置验证
#[test]
async fn test_forward_validation() {
    let config = create_test_config();

    // 测试有效的转发规则配置
    let valid_forward = ForwardConfig {
        name: "valid_forward".to_string(),
        port: 3000,
        address: "127.0.0.1".to_string(),
        upstream_group: "test_group".to_string(), // 已存在的上游组
        ratelimit: RateLimitConfig::default(),
        timeout: TimeoutConfig { connect: 5 },
    };

    // 验证有效的转发规则配置
    let result = config.validate_forward_config(&valid_forward);
    assert!(result.is_ok());

    // 测试引用不存在上游组的转发规则
    let invalid_ref_forward = ForwardConfig {
        name: "invalid_ref".to_string(),
        port: 3000,
        address: "127.0.0.1".to_string(),
        upstream_group: "nonexistent_group".to_string(), // 不存在的上游组
        ratelimit: RateLimitConfig::default(),
        timeout: TimeoutConfig { connect: 5 },
    };

    // 验证引用不存在上游组的转发规则
    let result = config.validate_forward_config(&invalid_ref_forward);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));

    // 测试无效端口的转发规则
    let invalid_port_forward = ForwardConfig {
        name: "invalid_port".to_string(),
        port: 0, // 无效端口
        address: "127.0.0.1".to_string(),
        upstream_group: "test_group".to_string(),
        ratelimit: RateLimitConfig::default(),
        timeout: TimeoutConfig { connect: 5 },
    };

    // 验证无效端口的转发规则
    let result = config.validate_forward_config(&invalid_port_forward);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("port"));

    // 测试无效地址的转发规则
    let invalid_address_forward = ForwardConfig {
        name: "invalid_address".to_string(),
        port: 3000,
        address: "".to_string(), // 空地址
        upstream_group: "test_group".to_string(),
        ratelimit: RateLimitConfig::default(),
        timeout: TimeoutConfig { connect: 5 },
    };

    // 验证无效地址的转发规则
    let result = config.validate_forward_config(&invalid_address_forward);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("address"));
}

/// 测试限流配置验证
#[test]
async fn test_ratelimit_validation() {
    let config = create_test_config();

    // 测试有效的限流配置
    let valid_ratelimit = RateLimitConfig {
        enabled: true,
        per_second: 100,
        burst: 200,
    };

    // 创建包含有效限流配置的转发规则
    let valid_forward = ForwardConfig {
        name: "valid_ratelimit".to_string(),
        port: 3000,
        address: "127.0.0.1".to_string(),
        upstream_group: "test_group".to_string(),
        ratelimit: valid_ratelimit,
        timeout: TimeoutConfig { connect: 5 },
    };

    // 验证有效的转发规则配置
    let result = config.validate_forward_config(&valid_forward);
    assert!(result.is_ok());

    // 测试无效的限流配置（per_second为0）
    let invalid_per_second_ratelimit = RateLimitConfig {
        enabled: true,
        per_second: 0, // 无效值
        burst: 200,
    };

    // 创建包含无效限流配置的转发规则
    let invalid_forward = ForwardConfig {
        name: "invalid_ratelimit".to_string(),
        port: 3000,
        address: "127.0.0.1".to_string(),
        upstream_group: "test_group".to_string(),
        ratelimit: invalid_per_second_ratelimit,
        timeout: TimeoutConfig { connect: 5 },
    };

    // 验证无效的转发规则配置
    let result = config.validate_forward_config(&invalid_forward);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("per_second"));

    // 测试无效的限流配置（burst超出有效范围）
    let invalid_burst_ratelimit = RateLimitConfig {
        enabled: true,
        per_second: 100,
        burst: 10000000, // 超出最大值
    };

    // 创建包含无效限流配置的转发规则
    let invalid_forward = ForwardConfig {
        name: "invalid_burst".to_string(),
        port: 3000,
        address: "127.0.0.1".to_string(),
        upstream_group: "test_group".to_string(),
        ratelimit: invalid_burst_ratelimit,
        timeout: TimeoutConfig { connect: 5 },
    };

    // 验证无效的转发规则配置
    let result = config.validate_forward_config(&invalid_forward);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("burst"));
}
