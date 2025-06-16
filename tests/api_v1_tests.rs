//! API v1 集成测试模块

// 导入测试所需的依赖
use axum::{
    body::to_bytes,
    http::{Method, StatusCode},
};
use llmproxy::{
    config::{self, Config, RateLimitConfig, TimeoutConfig, UpstreamConfig},
    *,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

// 辅助模块，包含测试设置和帮助函数
mod helpers {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use axum::Router;
    use llmproxy::{
        config::{serializer::SerializableArcString, ForwardConfig, HttpServerConfig}, // 修正SerializableArcString导入路径
    };
    use tower::ServiceExt;

    // TestApp 结构体，封装了测试环境
    pub struct TestApp {
        pub router: Router,
        pub config: Arc<RwLock<Config>>,
    }

    impl TestApp {
        // 辅助函数：发送 GET 请求
        pub async fn get(&mut self, path: &str) -> axum::response::Response {
            let request = Request::builder()
                .method(Method::GET)
                .uri(path)
                .body(Body::empty())
                .unwrap();

            println!("Sending GET request to: {}", path);
            let response = self.router.clone().oneshot(request).await.unwrap();
            println!("Response status: {}", response.status());
            response
        }

        // 辅助函数：发送 POST 请求
        pub async fn post(
            &mut self,
            path: &str,
            body: serde_json::Value,
        ) -> axum::response::Response {
            let request = Request::builder()
                .method(Method::POST)
                .uri(path)
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap();

            println!("Sending POST request to: {}", path);
            let response = self.router.clone().oneshot(request).await.unwrap();
            println!("Response status: {}", response.status());
            response
        }

        // 辅助函数：发送 PUT 请求
        pub async fn put(
            &mut self,
            path: &str,
            body: serde_json::Value,
        ) -> axum::response::Response {
            let request = Request::builder()
                .method(Method::PUT)
                .uri(path)
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap();

            println!("Sending PUT request to: {}", path);
            let response = self.router.clone().oneshot(request).await.unwrap();
            println!("Response status: {}", response.status());
            response
        }

        // 辅助函数：发送 PATCH 请求
        pub async fn patch(
            &mut self,
            path: &str,
            body: serde_json::Value,
        ) -> axum::response::Response {
            let request = Request::builder()
                .method(Method::PATCH)
                .uri(path)
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap();

            println!("Sending PATCH request to: {}", path);
            let response = self.router.clone().oneshot(request).await.unwrap();
            println!("Response status: {}", response.status());
            response
        }

        // 辅助函数：发送 DELETE 请求
        pub async fn delete(&mut self, path: &str) -> axum::response::Response {
            let request = Request::builder()
                .method(Method::DELETE)
                .uri(path)
                .body(Body::empty())
                .unwrap();

            println!("Sending DELETE request to: {}", path);
            let response = self.router.clone().oneshot(request).await.unwrap();
            println!("Response status: {}", response.status());
            response
        }
    }

    // 启动并配置测试应用实例
    pub async fn spawn_app() -> TestApp {
        // 创建一个用于测试的默认配置
        let config = Config {
            http_server: Some(HttpServerConfig {
                admin: config::AdminConfig {
                    ..Default::default()
                },
                forwards: vec![ForwardConfig {
                    name: "default_forward".to_string(),
                    address: "0.0.0.0".to_string(),
                    port: 8080,
                    upstream_group: "default_group".to_string(),
                    ratelimit: RateLimitConfig::default(),
                    timeout: TimeoutConfig::default(),
                }],
            }),
            upstreams: vec![config::UpstreamConfig {
                name: "default_upstream".to_string(),
                url: SerializableArcString::from("http://127.0.0.1:1".to_string()),
                auth: Some(config::AuthConfig {
                    r#type: config::AuthType::None,
                    token: None,
                    username: None,
                    password: None,
                }),
                weight: 1,
                http_client: config::HttpClientConfig::default(),
                headers: Vec::new(),
                breaker: None,
            }],
            upstream_groups: vec![config::UpstreamGroupConfig {
                name: "default_group".to_string(),
                upstreams: vec![config::UpstreamRef {
                    name: "default_upstream".to_string(),
                    weight: 100,
                }],
                balance: config::BalanceConfig::default(),
                http_client: config::HttpClientConfig::default(),
            }],
        };

        // 将配置包装在 Arc<RwLock<>> 中以实现共享和可变性
        let shared_config = Arc::new(RwLock::new(config));

        println!("Creating API routes with config");
        // 获取 API v1 路由并应用共享配置状态
        let app_router = api::v1::api_routes(shared_config.clone());
        println!("API routes created");

        // 返回 TestApp 实例
        TestApp {
            router: app_router,
            config: shared_config,
        }
    }
}

// Upstreams API 的测试模块
#[cfg(test)]
mod upstreams {
    use super::helpers::spawn_app;
    use super::*;
    use llmproxy::api::v1::{ErrorResponse, SuccessResponse};

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
        let success_response: SuccessResponse<UpstreamConfig> =
            serde_json::from_slice(&body).unwrap();
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
        assert_eq!(error_response.error.r#type, "Not Found");
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
        assert_eq!(error_response.error.r#type, "conflict");
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
        assert_eq!(error_response.error.r#type, "Not Found");
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
        assert_eq!(error_response.error.r#type, "dependency_conflict");
        assert!(error_response.error.message.contains("default_group"));
    }
}

// Upstream Groups API 的测试模块
#[cfg(test)]
mod upstream_groups {
    use super::helpers::spawn_app;
    use super::*;
    use llmproxy::api::v1::{ErrorResponse, SuccessResponse};
    use serde_json::Value;

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
        assert_eq!(error_response.error.r#type, "Not Found");
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
        assert_eq!(error_response.error.r#type, "validation_error");
        assert!(error_response.error.message.contains("not found"));
    }
}

// Forwards API 的测试模块
#[cfg(test)]
mod forwards {
    use super::helpers::spawn_app;
    use super::*;
    use llmproxy::{
        api::v1::{ErrorResponse, SuccessResponse},
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
        let success_response: SuccessResponse<ForwardConfig> =
            serde_json::from_slice(&body).unwrap();
        assert_eq!(
            success_response.data.as_ref().unwrap().name,
            "default_forward"
        );
    }

    #[tokio::test]
    async fn test_get_forward_not_found() {
        let mut app = spawn_app().await;
        let response = app.get("/api/v1/forwards/nonexistent").await;

        // 打印响应体
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8_lossy(&body);
        println!("Response body: {}", body_str);

        // 解析响应体
        let error_response: ErrorResponse = serde_json::from_str(&body_str).unwrap();
        assert_eq!(error_response.code, 404);
        assert_eq!(error_response.error.r#type, "Not Found");
    }
}
