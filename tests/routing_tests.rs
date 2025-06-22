use llmproxy::{
    config::{http_server::RoutingRule, ForwardConfig},
    error::AppError,
    server::router::Router,
};

// 注意：当前的路由实现是基本的精确路径匹配
// 未来可以扩展支持：
// 1. 命名参数 - 例如 /users/:id
// 2. 通配符匹配 - 例如 /files/*
// 3. 正则表达式匹配 - 例如 /items/([0-9]+)
// 这将需要替换简单的HashMap实现，使用更复杂的路径匹配算法，如RadixMap或其他路由树

/// 创建测试配置
fn create_test_forward_config() -> ForwardConfig {
    ForwardConfig {
        name: "test_forward".to_string(),
        port: 3000,
        address: "127.0.0.1".to_string(),
        default_group: "default".to_string(),
        routing: Some(vec![
            RoutingRule {
                path: "/api".to_string(),
                target_group: "api_group".to_string(),
            },
            RoutingRule {
                path: "/api/v1".to_string(),
                target_group: "v1_group".to_string(),
            },
        ]),
        ratelimit: None,
        timeout: None,
    }
}

/// 测试成功创建Router实例
#[test]
fn test_router_creation_success() {
    let config = create_test_forward_config();
    let router = Router::new(&config);
    assert!(router.is_ok());
}

/// 测试创建Router时处理重复路径的情况
#[test]
fn test_router_creation_duplicate_paths() {
    let mut config = create_test_forward_config();

    // 添加一个重复的路径
    if let Some(ref mut routing) = config.routing {
        routing.push(RoutingRule {
            path: "/api".to_string(),
            target_group: "duplicate_group".to_string(),
        });
    }

    let router = Router::new(&config);
    assert!(router.is_err());

    match router {
        Err(AppError::Config(msg)) => {
            assert!(msg.contains("Duplicate routing path found"));
        }
        _ => panic!("Expected Config error for duplicate path"),
    }
}

/// 测试精确路径匹配
#[test]
fn test_router_exact_path_match() {
    let config = create_test_forward_config();
    let router = Router::new(&config).unwrap();

    // 测试精确路径匹配
    let result = router.get_target_group("/api");
    assert_eq!(result.target_group, "api_group");
    assert!(!result.is_default);

    let result = router.get_target_group("/api/v1");
    assert_eq!(result.target_group, "v1_group");
    assert!(!result.is_default);
}

/// 测试没有匹配时回退到默认组
#[test]
fn test_router_no_match_fallback() {
    let config = create_test_forward_config();
    let router = Router::new(&config).unwrap();

    // 测试不匹配时的默认回退
    let result = router.get_target_group("/non_existent_path");
    assert_eq!(result.target_group, "default");
    assert!(result.is_default);

    let result = router.get_target_group("/api/v3"); // 不存在的API版本
    assert_eq!(result.target_group, "default");
    assert!(result.is_default);
}

/// 测试没有路由规则时的行为
#[test]
fn test_router_empty_routing_rules() {
    let config = ForwardConfig {
        name: "test_forward".to_string(),
        port: 3000,
        address: "127.0.0.1".to_string(),
        default_group: "default".to_string(),
        routing: None,
        ratelimit: None,
        timeout: None,
    };

    let router = Router::new(&config).unwrap();

    // 当没有路由规则时，所有请求都应该使用默认组
    let result = router.get_target_group("/any/path");
    assert_eq!(result.target_group, "default");
    assert!(result.is_default);
}

/// 测试路径变体
#[test]
fn test_router_path_variations() {
    let mut config = create_test_forward_config();

    // 添加更多路径变体
    if let Some(ref mut routing) = config.routing {
        routing.push(RoutingRule {
            path: "/".to_string(),
            target_group: "root_group".to_string(),
        });
        routing.push(RoutingRule {
            path: "/api/v1/users".to_string(),
            target_group: "users_group".to_string(),
        });
    }

    let router = Router::new(&config).unwrap();

    // 测试根路径
    let result = router.get_target_group("/");
    assert_eq!(result.target_group, "root_group");
    assert!(!result.is_default);

    // 测试嵌套路径
    let result = router.get_target_group("/api/v1/users");
    assert_eq!(result.target_group, "users_group");
    assert!(!result.is_default);

    // 测试不存在的嵌套路径
    let result = router.get_target_group("/api/v1/posts");
    assert_eq!(result.target_group, "default");
    assert!(result.is_default);
}
