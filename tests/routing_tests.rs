use llmproxy::{
    config::{http_server::RoutingRule, ForwardConfig},
    server::router::Router,
};

// ========== 精确路径匹配 ==========

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

/// 测试路由器创建成功
#[tokio::test]
async fn test_router_creation_success() {
    let config = create_test_forward_config();
    let result = Router::new(&config);
    assert!(result.is_ok());
}

/// 测试路由器创建失败 - 重复路径
#[tokio::test]
async fn test_router_creation_duplicate_paths() {
    let mut config = create_test_forward_config();
    if let Some(ref mut routing) = config.routing {
        routing.push(RoutingRule {
            path: "/api".to_string(), // 重复的路径
            target_group: "another_group".to_string(),
        });
    }

    let result = Router::new(&config);
    assert!(result.is_err());
}

/// 测试精确路径匹配
#[tokio::test]
async fn test_router_exact_path_match() {
    let config = create_test_forward_config();
    let router = Router::new(&config).unwrap();

    // 测试精确路径匹配
    let result = router.get_target_group("/api").await;
    assert_eq!(result.target_group, "api_group");
    assert!(!result.is_default);

    let result = router.get_target_group("/api/v1").await;
    assert_eq!(result.target_group, "v1_group");
    assert!(!result.is_default);
}

/// 测试没有匹配时回退到默认组
#[tokio::test]
async fn test_router_no_match_fallback() {
    let config = create_test_forward_config();
    let router = Router::new(&config).unwrap();

    // 测试不匹配时的默认回退
    let result = router.get_target_group("/non_existent_path").await;
    assert_eq!(result.target_group, "default");
    assert!(result.is_default);

    let result = router.get_target_group("/api/v3").await; // 不存在的API版本
    assert_eq!(result.target_group, "default");
    assert!(result.is_default);
}

/// 测试没有路由规则时的行为
#[tokio::test]
async fn test_router_empty_routing_rules() {
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
    let result = router.get_target_group("/any/path").await;
    assert_eq!(result.target_group, "default");
    assert!(result.is_default);
}

/// 测试路径变体
#[tokio::test]
async fn test_router_path_variations() {
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
    let result = router.get_target_group("/").await;
    assert_eq!(result.target_group, "root_group");
    assert!(!result.is_default);

    // 测试嵌套路径
    let result = router.get_target_group("/api/v1/users").await;
    assert_eq!(result.target_group, "users_group");
    assert!(!result.is_default);

    // 测试不存在的嵌套路径
    let result = router.get_target_group("/api/v1/posts").await;
    assert_eq!(result.target_group, "default");
    assert!(result.is_default);
}

// ========== 扩展路由功能的测试 ==========
// 扩展支持：
// 1. 命名参数 - 例如 /users/:id
// 2. 通配符匹配 - 例如 /files/*
// 3. 正则表达式匹配 - 例如 /items/([0-9]+)
// 这将需要替换简单的HashMap实现，使用更复杂的路径匹配算法，如RadixMap或其他路由树

/// 创建支持高级路由的测试配置
/// 这个函数假设未来的Router实现将支持命名参数、通配符和正则表达式
fn create_extended_routing_config() -> ForwardConfig {
    ForwardConfig {
        name: "extended_router".to_string(),
        port: 3000,
        address: "127.0.0.1".to_string(),
        default_group: "default".to_string(),
        routing: Some(vec![
            // 命名参数
            RoutingRule {
                path: "/users/:id".to_string(),
                target_group: "user_detail".to_string(),
            },
            RoutingRule {
                path: "/posts/:category/:id".to_string(),
                target_group: "categorized_post".to_string(),
            },
            // 通配符
            RoutingRule {
                path: "/files/*".to_string(),
                target_group: "file_server".to_string(),
            },
            RoutingRule {
                path: "/api/*/docs".to_string(),
                target_group: "api_docs".to_string(),
            },
            // 正则表达式
            RoutingRule {
                path: "/items/{id:[0-9]+}".to_string(),
                target_group: "item_by_id".to_string(),
            },
            // 注意：这里很蠢，他不支持 [A-Z]{3}\d{3} 这种正则表达式。是依赖库的问题
            RoutingRule {
                path: "/products/{code:[A-Z][A-Z][A-Z][0-9][0-9][0-9]}".to_string(),
                target_group: "product_by_code".to_string(),
            },
            // 混合模式
            RoutingRule {
                path: "/api/:version/users/{id:[0-9]+}/profile".to_string(),
                target_group: "user_profile".to_string(),
            },
        ]),
        ratelimit: None,
        timeout: None,
    }
}

/// 测试命名参数匹配
#[tokio::test]
async fn test_named_parameters_matching() {
    let config = create_extended_routing_config();
    // 注意：这假设未来的Router实现将支持命名参数
    let router = Router::new(&config).unwrap();

    // 基本参数匹配测试
    let result = router.get_target_group("/users/123").await;
    assert_eq!(result.target_group, "user_detail");
    assert!(!result.is_default);

    // 多参数匹配测试
    let result = router.get_target_group("/posts/tech/42").await;
    assert_eq!(result.target_group, "categorized_post");
    assert!(!result.is_default);

    // 不匹配的参数路径（参数不足）
    let result = router.get_target_group("/posts/tech").await;
    assert_eq!(result.target_group, "default");
    assert!(result.is_default);

    // 不匹配的参数路径（参数过多）
    let result = router.get_target_group("/users/123/extra").await;
    assert_eq!(result.target_group, "default");
    assert!(result.is_default);
}

/// 测试通配符匹配
#[tokio::test]
async fn test_wildcard_matching() {
    let config = create_extended_routing_config();
    let router = Router::new(&config).unwrap();

    // 基本通配符匹配
    let result = router.get_target_group("/files/document.pdf").await;
    assert_eq!(result.target_group, "file_server");
    assert!(!result.is_default);

    // 通配符匹配多级路径
    let result = router
        .get_target_group("/files/documents/report.docx")
        .await;
    assert_eq!(result.target_group, "file_server");
    assert!(!result.is_default);

    // 中间部分通配符匹配
    let result = router.get_target_group("/api/v1/docs").await;
    assert_eq!(result.target_group, "api_docs");
    assert!(!result.is_default);

    let result = router.get_target_group("/api/v2/docs").await;
    assert_eq!(result.target_group, "api_docs");
    assert!(!result.is_default);

    // 通配符不匹配
    let result = router.get_target_group("/api/v1/documents").await; // 不是 /docs 结尾
    assert_eq!(result.target_group, "default");
    assert!(result.is_default);
}

/// 测试正则表达式匹配
#[tokio::test]
async fn test_regex_matching() {
    let config = create_extended_routing_config();
    let router = Router::new(&config).unwrap();

    // 数字ID匹配
    let result = router.get_target_group("/items/42").await;
    assert_eq!(result.target_group, "item_by_id");
    assert!(!result.is_default);

    // 产品代码匹配（格式：3个大写字母+3个数字）
    let result = router.get_target_group("/products/ABC123").await;
    assert_eq!(result.target_group, "product_by_code");
    assert!(!result.is_default);

    // 不匹配的正则表达式
    let result = router.get_target_group("/items/abc").await; // 不是数字ID
    assert_eq!(result.target_group, "default");
    assert!(result.is_default);

    let result = router.get_target_group("/products/abc123").await; // 小写字母
    assert_eq!(result.target_group, "default");
    assert!(result.is_default);

    let result = router.get_target_group("/products/ABC12").await; // 数字不够
    assert_eq!(result.target_group, "default");
    assert!(result.is_default);
}

/// 测试路由优先级
#[tokio::test]
async fn test_routing_priority() {
    // 创建有重叠路由规则的配置
    let config = ForwardConfig {
        name: "priority_test".to_string(),
        port: 3000,
        address: "127.0.0.1".to_string(),
        default_group: "default".to_string(),
        routing: Some(vec![
            // 静态路径
            RoutingRule {
                path: "/api/users/admin".to_string(),
                target_group: "static_admin".to_string(),
            },
            // 命名参数
            RoutingRule {
                path: "/api/users/:id".to_string(),
                target_group: "user_param".to_string(),
            },
            // 通配符
            RoutingRule {
                path: "/api/*".to_string(),
                target_group: "api_wildcard".to_string(),
            },
        ]),
        ratelimit: None,
        timeout: None,
    };

    let router = Router::new(&config).unwrap();

    // 测试静态路径优先级
    let result = router.get_target_group("/api/users/admin").await;
    assert_eq!(result.target_group, "static_admin");
    assert!(!result.is_default);

    // 测试命名参数优先级
    let result = router.get_target_group("/api/users/123").await;
    assert_eq!(result.target_group, "user_param");
    assert!(!result.is_default);

    // 测试通配符优先级
    let result = router.get_target_group("/api/products").await;
    assert_eq!(result.target_group, "api_wildcard");
    assert!(!result.is_default);
}

/// 测试混合路由模式
#[tokio::test]
async fn test_mixed_routing_patterns() {
    let config = create_extended_routing_config();
    let router = Router::new(&config).unwrap();

    // 测试混合模式匹配
    let result = router.get_target_group("/api/v2/users/42/profile").await;
    assert_eq!(result.target_group, "user_profile");
    assert!(!result.is_default);

    // 测试混合模式不匹配
    let result = router.get_target_group("/api/v2/users/abc/profile").await; // 非数字ID
    assert_eq!(result.target_group, "default");
    assert!(result.is_default);
}
