//! Upstreams API 测试模块
use super::helpers::spawn_app;
use axum::{body::to_bytes, http::StatusCode};
use llmproxy::{
    api::v1::models::{ErrorResponse, SuccessResponse},
    config::UpstreamConfig,
};
use serde_json::json;

#[tokio::test]
async fn test_list_upstreams_success() {
    let mut app = spawn_app().await;
    let response = app.get("/api/v1/upstreams").await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let success_response: SuccessResponse<Vec<UpstreamConfig>> =
        serde_json::from_slice(&body).unwrap();
    assert!(!success_response.data.as_ref().unwrap().is_empty());
    assert_eq!(
        success_response.data.as_ref().unwrap()[0].name,
        "default_upstream"
    );
}

#[tokio::test]
async fn test_get_upstream_success() {
    let mut app = spawn_app().await;
    let response = app.get("/api/v1/upstreams/default_upstream").await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let success_response: SuccessResponse<UpstreamConfig> = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        success_response.data.as_ref().unwrap().name,
        "default_upstream"
    );
}

#[tokio::test]
async fn test_get_upstream_not_found() {
    let mut app = spawn_app().await;
    let response = app.get("/api/v1/upstreams/nonexistent").await;

    // 打印响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    println!("Response body: {}", body_str);

    // 解析响应体
    let error_response: ErrorResponse = serde_json::from_str(&body_str).unwrap();
    assert_eq!(error_response.code, 404);
    assert_eq!(error_response.error.r#type, "NotFound");
}

// 测试成功创建一个新的 Upstream
#[tokio::test]
async fn test_create_upstream_success() {
    let mut app = spawn_app().await;
    let upstream_payload = json!({
        "name": "test-upstream-1",
        "url": "http://localhost:8080",
        "auth": { "type": "none" }
    });

    let response = app.post("/api/v1/upstreams", upstream_payload).await;

    // 打印响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    println!("Response body: {}", body_str);

    // 检查是否成功创建
    if body_str.contains("error") {
        let error_response: ErrorResponse = serde_json::from_str(&body_str).unwrap();
        panic!(
            "Failed to create upstream: {}",
            error_response.error.message
        );
    } else {
        let success_response: SuccessResponse<UpstreamConfig> =
            serde_json::from_str(&body_str).unwrap();
        assert_eq!(success_response.code, 200);
        assert_eq!(
            success_response.data.as_ref().unwrap().name,
            "test-upstream-1"
        );
    }

    // 验证配置是否真的被更新
    let config = app.config.read().await;
    assert_eq!(config.upstreams.len(), 2); // 1 default + 1 new
    assert!(config.upstreams.iter().any(|u| u.name == "test-upstream-1"));
}

// 测试创建已存在的 Upstream 导致冲突
#[tokio::test]
async fn test_create_upstream_conflict() {
    let mut app = spawn_app().await;
    let upstream_payload = json!({
        "name": "default_upstream", // 这个已经存在
        "url": "http://localhost:8080"
    });

    let response = app.post("/api/v1/upstreams", upstream_payload).await;

    // 打印响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    println!("Response body: {}", body_str);

    // 解析响应体
    let error_response: ErrorResponse = serde_json::from_str(&body_str).unwrap();
    assert_eq!(error_response.code, 409);
    assert_eq!(error_response.error.r#type, "Conflict");
}

// 测试更新一个 Upstream
#[tokio::test]
async fn test_update_upstream_success() {
    let mut app = spawn_app().await;
    let updated_payload = json!({
        "name": "default_upstream",
        "url": "http://127.0.0.1:9999" // 更新地址
    });

    let response = app
        .put("/api/v1/upstreams/default_upstream", updated_payload)
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let config = app.config.read().await;
    let updated_upstream = config
        .upstreams
        .iter()
        .find(|u| u.name == "default_upstream")
        .unwrap();
    assert_eq!(
        updated_upstream.url.as_ref() as &str,
        "http://127.0.0.1:9999"
    );
}

// 测试更新一个不存在的 Upstream
#[tokio::test]
async fn test_update_upstream_not_found() {
    let mut app = spawn_app().await;
    let payload = json!({ "name": "nonexistent", "url": "http://127.0.0.1:1" });
    let response = app.put("/api/v1/upstreams/nonexistent", payload).await;

    // 打印响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    println!("Response body: {}", body_str);

    // 解析响应体
    let error_response: ErrorResponse = serde_json::from_str(&body_str).unwrap();
    assert_eq!(error_response.code, 404);
    assert_eq!(error_response.error.r#type, "NotFound");
}

// 测试成功删除一个 Upstream
#[tokio::test]
async fn test_delete_upstream_success() {
    let mut app = spawn_app().await;
    // 先创建一个不被依赖的 upstream
    let upstream_payload = json!({
        "name": "to_be_deleted",
        "url": "http://localhost:8081"
    });
    app.post("/api/v1/upstreams", upstream_payload).await;

    // 然后删除它
    let response = app.delete("/api/v1/upstreams/to_be_deleted").await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let config = app.config.read().await;
    assert!(!config.upstreams.iter().any(|u| u.name == "to_be_deleted"));
}

// 测试删除一个正在被 UpstreamGroup 使用的 Upstream
#[tokio::test]
async fn test_delete_upstream_in_use_conflict() {
    let mut app = spawn_app().await;

    // "default_upstream" is used by "default_group"
    let response = app.delete("/api/v1/upstreams/default_upstream").await;

    // 打印响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    println!("Response body: {}", body_str);

    // 解析响应体
    let error_response: ErrorResponse = serde_json::from_str(&body_str).unwrap();
    assert_eq!(error_response.code, 409);
    assert_eq!(error_response.error.r#type, "Conflict");
    assert!(error_response.error.message.contains("default_group"));
}
