//! Forwards API 测试模块
use super::helpers::spawn_app;
use axum::{body::to_bytes, http::StatusCode};
use llmproxy::{
    api::v1::models::{ErrorResponse, SuccessResponse},
    config::ForwardConfig,
};

#[tokio::test]
async fn test_list_forwards_success() {
    let mut app = spawn_app().await;
    let response = app.get("/api/v1/forwards").await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let success_response: SuccessResponse<Vec<ForwardConfig>> =
        serde_json::from_slice(&body).unwrap();
    assert!(!success_response.data.as_ref().unwrap().is_empty());
    assert_eq!(
        success_response.data.as_ref().unwrap()[0].name,
        "default_forward"
    );
}

#[tokio::test]
async fn test_get_forward_success() {
    let mut app = spawn_app().await;
    let response = app.get("/api/v1/forwards/default_forward").await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let success_response: SuccessResponse<ForwardConfig> = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        success_response.data.as_ref().unwrap().name,
        "default_forward"
    );
}

#[tokio::test]
async fn test_get_forward_not_found() {
    let mut app = spawn_app().await;
    let response = app.get("/api/v1/forwards/nonexistent").await;

    // 获取响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);

    // 解析响应体
    let error_response: ErrorResponse = serde_json::from_str(&body_str).unwrap();
    assert_eq!(error_response.code, 404);
    assert_eq!(error_response.error.r#type, "NotFound");
}
