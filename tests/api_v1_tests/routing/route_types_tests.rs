//! 路由规则类型 API 测试

use super::*;

#[tokio::test]
async fn test_different_route_path_types() {
    // 初始化测试环境
    let mut app = spawn_app().await;
    setup_test_upstream_groups(&mut app).await;

    let forward_name = "default_forward";
    let target_group = "test_group";

    // 1. 静态路径
    let static_path = "/api/users/admin";
    let payload = json!({
        "path": static_path,
        "target_group": target_group
    });
    let response = app
        .post(
            &format!("/api/v1/forwards/{}/routes", forward_name),
            payload,
        )
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // 2. 命名参数路径
    let named_param_path = "/api/users/:id";
    let payload = json!({
        "path": named_param_path,
        "target_group": target_group
    });
    let response = app
        .post(
            &format!("/api/v1/forwards/{}/routes", forward_name),
            payload,
        )
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // 3. 带正则的参数路径
    let regex_param_path = "/api/items/{id:[0-9]+}";
    let payload = json!({
        "path": regex_param_path,
        "target_group": target_group
    });
    let response = app
        .post(
            &format!("/api/v1/forwards/{}/routes", forward_name),
            payload,
        )
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // 4. 复杂正则表达式
    let complex_regex_path = "/api/products/{code:[A-Z][A-Z][A-Z][0-9][0-9][0-9]}";
    let payload = json!({
        "path": complex_regex_path,
        "target_group": target_group
    });
    let response = app
        .post(
            &format!("/api/v1/forwards/{}/routes", forward_name),
            payload,
        )
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // 5. 中间通配符
    let middle_wildcard_path = "/api/*/docs";
    let payload = json!({
        "path": middle_wildcard_path,
        "target_group": target_group
    });
    let response = app
        .post(
            &format!("/api/v1/forwards/{}/routes", forward_name),
            payload,
        )
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // 6. 混合模式
    let mixed_path = "/api/:version/users/{id:[0-9]+}/profile";
    let payload = json!({
        "path": mixed_path,
        "target_group": target_group
    });
    let response = app
        .post(
            &format!("/api/v1/forwards/{}/routes", forward_name),
            payload,
        )
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // 7. 尾部通配符
    let tail_wildcard_path = "/files/*";
    let payload = json!({
        "path": tail_wildcard_path,
        "target_group": target_group
    });
    let response = app
        .post(
            &format!("/api/v1/forwards/{}/routes", forward_name),
            payload,
        )
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // 验证所有路由规则是否都成功创建
    let response = app
        .get(&format!("/api/v1/forwards/{}/routes", forward_name))
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let success_response: SuccessResponse<Vec<RoutingRule>> =
        serde_json::from_slice(&body).unwrap();

    let routes = success_response.data.unwrap();

    // 验证每种类型的路由规则是否存在
    assert!(routes.iter().any(|r| r.path == static_path));
    assert!(routes.iter().any(|r| r.path == named_param_path));
    assert!(routes.iter().any(|r| r.path == regex_param_path));
    assert!(routes.iter().any(|r| r.path == complex_regex_path));
    assert!(routes.iter().any(|r| r.path == middle_wildcard_path));
    assert!(routes.iter().any(|r| r.path == mixed_path));
    assert!(routes.iter().any(|r| r.path == tail_wildcard_path));
}
