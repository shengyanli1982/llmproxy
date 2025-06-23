//! 路由规则 API 基本功能测试

use super::*;

#[tokio::test]
async fn test_list_routes_success() {
    // 初始化测试环境
    let mut app = spawn_app().await;
    setup_test_upstream_groups(&mut app).await;

    // 添加测试路由规则
    let forward_name = "default_forward";
    let path = "/api/test";
    let target_group = "test_group";

    add_test_route(&mut app, forward_name, path, target_group).await;

    // 发送请求
    let response = app
        .get(&format!("/api/v1/forwards/{}/routes", forward_name))
        .await;

    // 验证响应
    assert_eq!(response.status(), StatusCode::OK);

    // 解析响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let success_response: SuccessResponse<Vec<RoutingRule>> =
        serde_json::from_slice(&body).unwrap();

    // 验证响应内容
    assert!(success_response.data.is_some());
    let routes = success_response.data.unwrap();
    assert!(!routes.is_empty());

    // 确认我们的测试路由规则存在
    let test_route = routes.iter().find(|r| r.path == path);
    assert!(test_route.is_some());
    assert_eq!(test_route.unwrap().target_group, target_group);
}

#[tokio::test]
async fn test_get_route_success() {
    // 初始化测试环境
    let mut app = spawn_app().await;
    setup_test_upstream_groups(&mut app).await;

    // 添加测试路由规则
    let forward_name = "default_forward";
    let path = "/api/test/get";
    let target_group = "test_group";

    add_test_route(&mut app, forward_name, path, target_group).await;

    // 对路径进行 base64 编码
    let encoded_path = encode_path_to_base64(path);

    // 发送请求
    let response = app
        .get(&format!(
            "/api/v1/forwards/{}/routes/{}",
            forward_name, encoded_path
        ))
        .await;

    // 验证响应
    assert_eq!(response.status(), StatusCode::OK);

    // 解析响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let success_response: SuccessResponse<RoutingRule> = serde_json::from_slice(&body).unwrap();

    // 验证响应内容
    assert!(success_response.data.is_some());
    let route = success_response.data.unwrap();
    assert_eq!(route.path, path);
    assert_eq!(route.target_group, target_group);
}

#[tokio::test]
async fn test_create_route_success() {
    // 初始化测试环境
    let mut app = spawn_app().await;
    setup_test_upstream_groups(&mut app).await;

    // 准备请求数据
    let forward_name = "default_forward";
    let path = "/api/test/create";
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
async fn test_update_route_success() {
    // 初始化测试环境
    let mut app = spawn_app().await;
    setup_test_upstream_groups(&mut app).await;

    // 添加测试路由规则
    let forward_name = "default_forward";
    let path = "/api/test/update";
    let target_group = "test_group";
    let new_target_group = "another_group";

    add_test_route(&mut app, forward_name, path, target_group).await;

    // 对路径进行 base64 编码
    let encoded_path = encode_path_to_base64(path);

    // 准备更新请求数据
    let payload = json!({
        "target_group": new_target_group
    });

    // 发送更新请求
    let response = app
        .put(
            &format!("/api/v1/forwards/{}/routes/{}", forward_name, encoded_path),
            payload,
        )
        .await;

    // 验证响应
    assert_eq!(response.status(), StatusCode::OK);

    // 解析响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let success_response: SuccessResponse<RoutingRule> = serde_json::from_slice(&body).unwrap();

    // 验证响应内容
    assert!(success_response.data.is_some());
    let route = success_response.data.unwrap();
    assert_eq!(route.path, path);
    assert_eq!(route.target_group, new_target_group);

    // 再次获取该路由，确保已成功更新
    let get_response = app
        .get(&format!(
            "/api/v1/forwards/{}/routes/{}",
            forward_name, encoded_path
        ))
        .await;
    assert_eq!(get_response.status(), StatusCode::OK);

    let body = to_bytes(get_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let get_success_response: SuccessResponse<RoutingRule> = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        get_success_response.data.unwrap().target_group,
        new_target_group
    );
}

#[tokio::test]
async fn test_delete_route_success() {
    // 初始化测试环境
    let mut app = spawn_app().await;
    setup_test_upstream_groups(&mut app).await;

    // 添加测试路由规则
    let forward_name = "default_forward";
    let path = "/api/test/delete";
    let target_group = "test_group";

    add_test_route(&mut app, forward_name, path, target_group).await;

    // 对路径进行 base64 编码
    let encoded_path = encode_path_to_base64(path);

    // 发送删除请求
    let response = app
        .delete(&format!(
            "/api/v1/forwards/{}/routes/{}",
            forward_name, encoded_path
        ))
        .await;

    // 验证响应
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // 尝试获取已删除的路由，应该返回404
    let get_response = app
        .get(&format!(
            "/api/v1/forwards/{}/routes/{}",
            forward_name, encoded_path
        ))
        .await;
    assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
}
