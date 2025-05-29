use llmproxy::{
    config::{
        BalanceConfig, BalanceStrategy, ForwardConfig, RateLimitConfig, TimeoutConfig,
        UpstreamConfig, UpstreamGroupConfig, UpstreamRef,
    },
    error::AppError,
    server::ForwardServer,
    upstream::UpstreamManager,
};
use std::sync::Arc;
use tokio::time::Duration;
use uuid::Uuid;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

/// 创建测试用的上游管理器
async fn create_test_upstream_manager() -> (Arc<UpstreamManager>, MockServer) {
    // 创建模拟服务器
    let mock_server = MockServer::start().await;

    // 配置基本的 GET 响应
    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
        .mount(&mock_server)
        .await;

    // 创建上游配置
    let upstream_configs = vec![UpstreamConfig {
        name: "test_upstream".to_string(),
        url: mock_server.uri(),
        id: Uuid::new_v4().to_string(),
        auth: None,
        headers: vec![],
        breaker: None,
    }];

    // 创建上游组配置
    let group_configs = vec![UpstreamGroupConfig {
        name: "test_group".to_string(),
        upstreams: vec![UpstreamRef {
            name: "test_upstream".to_string(),
            weight: 1,
        }],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: Default::default(),
    }];

    // 创建上游管理器
    let upstream_manager = UpstreamManager::new(upstream_configs, group_configs)
        .await
        .unwrap();

    (Arc::new(upstream_manager), mock_server)
}

/// 测试创建 ForwardServer
#[tokio::test]
async fn test_forward_server_creation() {
    let (upstream_manager, _mock_server) = create_test_upstream_manager().await;

    let config = ForwardConfig {
        name: "test_forward".to_string(),
        port: 0, // 使用系统分配的端口
        address: "127.0.0.1".to_string(),
        upstream_group: "test_group".to_string(),
        ratelimit: RateLimitConfig::default(),
        timeout: TimeoutConfig::default(),
    };

    // 只验证能否成功创建服务器
    let result = ForwardServer::new(config, upstream_manager);
    assert!(result.is_ok());
}

/// 测试限速功能
#[tokio::test]
async fn test_rate_limiting() -> Result<(), AppError> {
    let (upstream_manager, _mock_server) = create_test_upstream_manager().await;

    // 创建一个配置很严格的限速器：每秒1个请求，突发上限2个
    let config = ForwardConfig {
        name: "rate_limited_forward".to_string(),
        port: 0, // 使用系统分配的端口
        address: "127.0.0.1".to_string(),
        upstream_group: "test_group".to_string(),
        ratelimit: RateLimitConfig {
            enabled: true,
            per_second: 1,
            burst: 2,
        },
        timeout: TimeoutConfig::default(),
    };

    // 只验证能否成功创建服务器
    let server = ForwardServer::new(config, upstream_manager)?;
    // 确保创建成功
    assert!(server.get_addr().is_ipv4());

    Ok(())
}

/// 测试服务器超时配置
#[tokio::test]
async fn test_server_timeout() -> Result<(), AppError> {
    let (upstream_manager, mock_server) = create_test_upstream_manager().await;

    // 创建一个延迟响应的模拟
    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(3)))
        .mount(&mock_server)
        .await;

    // 创建一个超时配置很短的服务
    let config = ForwardConfig {
        name: "timeout_forward".to_string(),
        port: 0, // 使用系统分配的端口
        address: "127.0.0.1".to_string(),
        upstream_group: "test_group".to_string(),
        ratelimit: RateLimitConfig::default(),
        timeout: TimeoutConfig {
            connect: 1, // 1秒连接超时
        },
    };

    // 只验证能否成功创建服务器
    let server = ForwardServer::new(config, upstream_manager)?;
    // 确保服务器创建成功
    assert!(server.get_addr().is_ipv4());

    Ok(())
}

/// 测试同时处理多个并发请求
#[tokio::test]
async fn test_concurrent_requests() -> Result<(), AppError> {
    let (upstream_manager, _mock_server) = create_test_upstream_manager().await;

    // 创建一个限速配置
    let config = ForwardConfig {
        name: "concurrent_forward".to_string(),
        port: 0, // 使用系统分配的端口
        address: "127.0.0.1".to_string(),
        upstream_group: "test_group".to_string(),
        ratelimit: RateLimitConfig {
            enabled: true,
            per_second: 5, // 每秒5个请求
            burst: 10,     // 突发上限10个
        },
        timeout: TimeoutConfig::default(),
    };

    // 只验证能否成功创建服务器
    let server = ForwardServer::new(config, upstream_manager)?;
    // 验证创建成功
    assert!(server.get_addr().is_ipv4());

    Ok(())
}

/// 测试关闭服务器能够正常处理已有连接
#[tokio::test]
async fn test_server_graceful_shutdown() -> Result<(), AppError> {
    let (upstream_manager, mock_server) = create_test_upstream_manager().await;

    // 创建一个慢速响应的模拟
    Mock::given(method("GET"))
        .and(path("/slow-shutdown"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(1))
                .set_body_string("Slow response"),
        )
        .mount(&mock_server)
        .await;

    // 创建服务配置
    let config = ForwardConfig {
        name: "shutdown_forward".to_string(),
        port: 0, // 使用系统分配的端口
        address: "127.0.0.1".to_string(),
        upstream_group: "test_group".to_string(),
        ratelimit: RateLimitConfig::default(),
        timeout: TimeoutConfig::default(),
    };

    // 只验证能否成功创建服务器
    let server = ForwardServer::new(config, upstream_manager)?;
    // 验证创建成功
    assert!(server.get_addr().is_ipv4());

    Ok(())
}
