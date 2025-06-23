//! 路由规则 API 极端情况测试

use super::*;

#[tokio::test]
async fn test_create_route_empty_path() {
    // 初始化测试环境
    let mut app = spawn_app().await;
    setup_test_upstream_groups(&mut app).await;

    // 准备请求数据，使用空路径
    let forward_name = "default_forward";
    let empty_path = "";
    let target_group = "test_group";

    let payload = json!({
        "path": empty_path,
        "target_group": target_group
    });

    // 发送请求
    let response = app
        .post(
            &format!("/api/v1/forwards/{}/routes", forward_name),
            payload,
        )
        .await;

    // 验证响应
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // 解析响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let error_response: ErrorResponse = serde_json::from_slice(&body).unwrap();

    // 验证错误信息
    assert_eq!(error_response.error.r#type, "BadRequest");
    assert!(error_response.error.message.contains("Path"));
    assert!(error_response.error.message.contains("empty"));
}

#[tokio::test]
async fn test_create_route_empty_target_group() {
    // 初始化测试环境
    let mut app = spawn_app().await;

    // 准备请求数据，使用空目标组
    let forward_name = "default_forward";
    let path = "/api/test/empty_group";
    let empty_target_group = "";

    let payload = json!({
        "path": path,
        "target_group": empty_target_group
    });

    // 发送请求
    let response = app
        .post(
            &format!("/api/v1/forwards/{}/routes", forward_name),
            payload,
        )
        .await;

    // 验证响应
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // 解析响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let error_response: ErrorResponse = serde_json::from_slice(&body).unwrap();

    // 验证错误信息
    assert_eq!(error_response.error.r#type, "BadRequest");
    assert!(error_response.error.message.contains("Target group"));
    assert!(error_response.error.message.contains("empty"));
}

#[tokio::test]
async fn test_create_route_special_chars() {
    // 初始化测试环境
    let mut app = spawn_app().await;
    setup_test_upstream_groups(&mut app).await;

    // 准备请求数据，使用包含特殊字符的路径
    let forward_name = "default_forward";
    let path = "/api/test/special!@#$%^&*()_+";
    let target_group = "test_group";

    let payload = json!({
        "path": path,
        "target_group": target_group
    });

    // 发送请求
    let response = app
        .post(
            &format!("/api/v1/forwards/{}/routes", forward_name),
            payload,
        )
        .await;

    // 验证响应
    assert_eq!(response.status(), StatusCode::CREATED);

    // 解析响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let success_response: SuccessResponse<RoutingRule> = serde_json::from_slice(&body).unwrap();

    // 验证响应内容
    assert!(success_response.data.is_some());
    let route = success_response.data.unwrap();
    assert_eq!(route.path, path);
    assert_eq!(route.target_group, target_group);

    // 再次获取该路由，确保已成功创建
    let encoded_path = encode_path_to_base64(path);
    let get_response = app
        .get(&format!(
            "/api/v1/forwards/{}/routes/{}",
            forward_name, encoded_path
        ))
        .await;
    assert_eq!(get_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_create_route_very_long_path() {
    // 初始化测试环境
    let mut app = spawn_app().await;
    setup_test_upstream_groups(&mut app).await;

    // 准备请求数据，使用超长路径
    let forward_name = "default_forward";
    let path = format!("/api/test/{}", "a".repeat(2000));
    let target_group = "test_group";

    let payload = json!({
        "path": path,
        "target_group": target_group
    });

    // 发送请求
    let response = app
        .post(
            &format!("/api/v1/forwards/{}/routes", forward_name),
            payload,
        )
        .await;

    // 我们这里不断言具体的响应状态码，因为这依赖于实现
    // 无论成功还是失败，我们只需确保它不会崩溃
    assert!(response.status().is_client_error() || response.status().is_success());
}
