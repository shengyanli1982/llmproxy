//! 路由规则 API 错误路径测试

use super::*;

#[tokio::test]
async fn test_list_routes_forward_not_found() {
    // 初始化测试环境
    let mut app = spawn_app().await;

    // 发送请求获取不存在的转发服务的路由规则
    let response = app.get("/api/v1/forwards/nonexistent_forward/routes").await;

    // 验证响应
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // 解析响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let error_response: ErrorResponse = serde_json::from_slice(&body).unwrap();

    // 验证错误信息
    assert_eq!(error_response.error.r#type, "NotFound");
    assert!(error_response.error.message.contains("nonexistent_forward"));
}

#[tokio::test]
async fn test_get_route_not_found() {
    // 初始化测试环境
    let mut app = spawn_app().await;

    let forward_name = "default_forward";
    let path = "/api/nonexistent/route";
    let encoded_path = encode_path_to_base64(path);

    // 发送请求获取不存在的路由规则
    let response = app
        .get(&format!(
            "/api/v1/forwards/{}/routes/{}",
            forward_name, encoded_path
        ))
        .await;

    // 验证响应
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // 解析响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let error_response: ErrorResponse = serde_json::from_slice(&body).unwrap();

    // 验证错误信息
    assert_eq!(error_response.error.r#type, "NotFound");
    assert!(error_response.error.message.contains("does not exist"));
}

#[tokio::test]
async fn test_get_route_invalid_base64() {
    // 初始化测试环境
    let mut app = spawn_app().await;

    let forward_name = "default_forward";
    let invalid_encoded_path = "this-is-not-valid-base64!@#";

    // 发送请求使用无效的base64编码路径
    let response = app
        .get(&format!(
            "/api/v1/forwards/{}/routes/{}",
            forward_name, invalid_encoded_path
        ))
        .await;

    // 验证响应
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // 解析响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let error_response: ErrorResponse = serde_json::from_slice(&body).unwrap();

    // 验证错误信息
    assert_eq!(error_response.error.r#type, "BadRequest");
    assert!(error_response.error.message.contains("Invalid base64"));
}

#[tokio::test]
async fn test_create_route_upstream_group_not_exist() {
    // 初始化测试环境
    let mut app = spawn_app().await;

    // 准备请求数据，使用不存在的上游组
    let forward_name = "default_forward";
    let path = "/api/test/bad_group";
    let nonexistent_group = "nonexistent_group";

    let payload = json!({
        "path": path,
        "target_group": nonexistent_group
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
    assert!(error_response.error.message.contains(nonexistent_group));
    assert!(error_response.error.message.contains("does not exist"));
}

#[tokio::test]
async fn test_create_route_conflict() {
    // 初始化测试环境
    let mut app = spawn_app().await;
    setup_test_upstream_groups(&mut app).await;

    // 添加测试路由规则
    let forward_name = "default_forward";
    let path = "/api/test/conflict";
    let target_group = "test_group";

    add_test_route(&mut app, forward_name, path, target_group).await;

    // 准备请求数据，使用已存在的路径
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
    assert_eq!(response.status(), StatusCode::CONFLICT);

    // 解析响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let error_response: ErrorResponse = serde_json::from_slice(&body).unwrap();

    // 验证错误信息
    assert_eq!(error_response.error.r#type, "Conflict");
    assert!(error_response.error.message.contains("already exists"));
}

#[tokio::test]
async fn test_update_route_not_found() {
    // 初始化测试环境
    let mut app = spawn_app().await;
    setup_test_upstream_groups(&mut app).await;

    // 不存在的路由规则路径
    let forward_name = "default_forward";
    let path = "/api/nonexistent/route/update";
    let encoded_path = encode_path_to_base64(path);
    let target_group = "test_group";

    // 准备更新请求数据
    let payload = json!({
        "target_group": target_group
    });

    // 发送更新请求
    let response = app
        .put(
            &format!("/api/v1/forwards/{}/routes/{}", forward_name, encoded_path),
            payload,
        )
        .await;

    // 验证响应
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // 解析响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let error_response: ErrorResponse = serde_json::from_slice(&body).unwrap();

    // 验证错误信息
    assert_eq!(error_response.error.r#type, "NotFound");
    assert!(error_response.error.message.contains("does not exist"));
}

#[tokio::test]
async fn test_delete_route_not_found() {
    // 初始化测试环境
    let mut app = spawn_app().await;

    // 不存在的路由规则路径
    let forward_name = "default_forward";
    let path = "/api/nonexistent/route/delete";
    let encoded_path = encode_path_to_base64(path);

    // 发送删除请求
    let response = app
        .delete(&format!(
            "/api/v1/forwards/{}/routes/{}",
            forward_name, encoded_path
        ))
        .await;

    // 验证响应
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // 解析响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let error_response: ErrorResponse = serde_json::from_slice(&body).unwrap();

    // 验证错误信息
    assert_eq!(error_response.error.r#type, "NotFound");
    assert!(error_response.error.message.contains("does not exist"));
}
