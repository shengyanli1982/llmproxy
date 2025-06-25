//! Upstream Groups API 测试模块
use super::helpers::spawn_app;
use axum::{body::to_bytes, http::StatusCode};
use llmproxy::api::v1::models::{ErrorResponse, SuccessResponse};
use serde_json::{json, Value};

#[tokio::test]
async fn test_list_upstream_groups_success() {
    let mut app = spawn_app().await;
    let response = app.get("/api/v1/upstream-groups").await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    println!("Response body: {}", body_str);

    let success_response: SuccessResponse<Value> = serde_json::from_str(&body_str).unwrap();
    let data = success_response.data.as_ref().unwrap();
    let groups = data.as_array().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["name"], "default_group");
    assert!(groups[0]["upstreams"][0]["name"].is_string());
}

#[tokio::test]
async fn test_get_upstream_group_success() {
    let mut app = spawn_app().await;
    let response = app.get("/api/v1/upstream-groups/default_group").await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    println!("Response body: {}", body_str);

    let success_response: SuccessResponse<Value> = serde_json::from_str(&body_str).unwrap();
    let data = success_response.data.as_ref().unwrap();
    assert_eq!(data["name"], "default_group");
    assert_eq!(data["upstreams"][0]["name"], "default_upstream");
}

#[tokio::test]
async fn test_get_upstream_group_not_found() {
    let mut app = spawn_app().await;
    let response = app.get("/api/v1/upstream-groups/nonexistent").await;

    // 打印响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    println!("Response body: {}", body_str);

    // 解析响应体
    let error_response: ErrorResponse = serde_json::from_str(&body_str).unwrap();
    assert_eq!(error_response.code, 404);
    assert_eq!(error_response.error.r#type, "NotFound");
}

// 测试成功更新一个 Upstream Group
#[tokio::test]
async fn test_patch_upstream_group_success() {
    let mut app = spawn_app().await;

    // 先创建一个新的 upstream 供测试使用
    let new_upstream_payload = json!({
        "name": "new-for-group-test",
        "url": "http://127.0.0.1:2"
    });
    app.post("/api/v1/upstreams", new_upstream_payload).await;

    // 更新 group
    let patch_payload = json!({
        "upstreams": [
            { "name": "new-for-group-test", "weight": 1 }
        ]
    });

    let response = app
        .patch("/api/v1/upstream-groups/default_group", patch_payload)
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    println!("Response body: {}", body_str);

    let success_response: SuccessResponse<Value> = serde_json::from_str(&body_str).unwrap();

    // 验证响应
    let group_data = &success_response.data.as_ref().unwrap();
    assert_eq!(group_data["name"], "default_group");
    let upstreams_in_response = group_data["upstreams"].as_array().unwrap();
    assert_eq!(upstreams_in_response.len(), 1);
    assert_eq!(upstreams_in_response[0]["name"], "new-for-group-test");
    assert_eq!(upstreams_in_response[0]["weight"], 1);

    // 验证配置
    let config = app.config.read().await;
    let group = config
        .upstream_groups
        .iter()
        .find(|g| g.name == "default_group")
        .unwrap();
    assert_eq!(group.upstreams[0].name, "new-for-group-test");
    assert_eq!(group.upstreams[0].weight, 1);
}

// 测试更新 Upstream Group 时引用一个不存在的 Upstream
#[tokio::test]
async fn test_patch_upstream_group_with_nonexistent_upstream() {
    let mut app = spawn_app().await;
    let payload = json!({"upstreams": [{"name": "nonexistent_upstream", "weight": 100}]});
    let response = app
        .patch("/api/v1/upstream-groups/default_group", payload)
        .await;

    // 打印响应体
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    println!("Response body: {}", body_str);

    // 解析响应体
    let error_response: ErrorResponse = serde_json::from_str(&body_str).unwrap();
    assert_eq!(error_response.code, 400);
    assert_eq!(error_response.error.r#type, "BadRequest");
    assert!(error_response.error.message.contains("not found"));
}

#[tokio::test]
#[ignore] // 由于API服务器没有真正启动，暂时忽略此测试
async fn test_patch_upstream_group_updates_load_balancer() {
    // 创建测试应用
    let test_app = spawn_app().await;
    let client = reqwest::Client::new();

    // 创建上游
    let upstream1 = json!({
        "name": "test_upstream1",
        "url": "http://localhost:8001/test",
        "weight": 1
    });

    let upstream2 = json!({
        "name": "test_upstream2",
        "url": "http://localhost:8002/test",
        "weight": 1
    });

    let upstream3 = json!({
        "name": "test_upstream3",
        "url": "http://localhost:8003/test",
        "weight": 2
    });

    client
        .post(&format!("{}/api/v1/upstreams", test_app.address))
        .json(&upstream1)
        .send()
        .await
        .expect("Failed to create upstream1");

    client
        .post(&format!("{}/api/v1/upstreams", test_app.address))
        .json(&upstream2)
        .send()
        .await
        .expect("Failed to create upstream2");

    client
        .post(&format!("{}/api/v1/upstreams", test_app.address))
        .json(&upstream3)
        .send()
        .await
        .expect("Failed to create upstream3");

    // 创建上游组
    let group = json!({
        "name": "test_group",
        "upstreams": [
            {
                "name": "test_upstream1",
                "weight": 1
            },
            {
                "name": "test_upstream2",
                "weight": 1
            }
        ],
        "balance": {
            "strategy": "RoundRobin"
        }
    });

    client
        .post(&format!("{}/api/v1/upstream_groups", test_app.address))
        .json(&group)
        .send()
        .await
        .expect("Failed to create group");

    // 获取初始组配置
    let initial_response = client
        .get(&format!(
            "{}/api/v1/upstream_groups/test_group",
            test_app.address
        ))
        .send()
        .await
        .expect("Failed to get group");

    let initial_response_text = initial_response
        .text()
        .await
        .expect("Failed to get response text");
    let initial_group: Value =
        serde_json::from_str(&initial_response_text).expect("Failed to parse initial group JSON");
    // 获取data字段中的内容
    let initial_group = &initial_group["data"];
    assert_eq!(initial_group["upstreams"].as_array().unwrap().len(), 2);

    // 更新组配置，使用不同的上游
    let update = json!({
        "upstreams": [
            {
                "name": "test_upstream3",
                "weight": 2
            }
        ]
    });

    client
        .patch(&format!(
            "{}/api/v1/upstream_groups/test_group",
            test_app.address
        ))
        .json(&update)
        .send()
        .await
        .expect("Failed to update group");

    // 获取更新后的组配置
    let updated_response = client
        .get(&format!(
            "{}/api/v1/upstream_groups/test_group",
            test_app.address
        ))
        .send()
        .await
        .expect("Failed to get updated group");

    let updated_response_text = updated_response
        .text()
        .await
        .expect("Failed to get response text");
    let updated_group: Value =
        serde_json::from_str(&updated_response_text).expect("Failed to parse updated group JSON");
    // 获取data字段中的内容
    let updated_group = &updated_group["data"];
    assert_eq!(updated_group["upstreams"].as_array().unwrap().len(), 1);
    assert_eq!(
        updated_group["upstreams"][0]["name"].as_str().unwrap(),
        "test_upstream3"
    );
    assert_eq!(updated_group["upstreams"][0]["weight"].as_i64().unwrap(), 2);

    // 验证负载均衡器已更新
    // 注意：这里我们只能通过间接方式验证，因为我们无法直接访问负载均衡器的内部状态
    // 在实际应用中，我们可以通过检查转发请求是否正确路由到新的上游来验证
}
