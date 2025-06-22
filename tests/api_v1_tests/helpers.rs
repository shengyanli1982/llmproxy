//! API v1 测试辅助模块

use axum::{
    body::Body,
    http::{Method, Request},
    response::Response,
    Router,
};
use llmproxy::{
    api::v1,
    config::{
        self, serializer::SerializableArcString, Config, ForwardConfig, HttpServerConfig,
        TimeoutConfig,
    },
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

// TestApp 结构体，封装了测试环境
pub struct TestApp {
    pub router: Router,
    pub config: Arc<RwLock<Config>>,
}

impl TestApp {
    // 辅助函数：发送 GET 请求
    pub async fn get(&mut self, path: &str) -> Response {
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
    pub async fn post(&mut self, path: &str, body: serde_json::Value) -> Response {
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
    pub async fn put(&mut self, path: &str, body: serde_json::Value) -> Response {
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
    pub async fn patch(&mut self, path: &str, body: serde_json::Value) -> Response {
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
    pub async fn delete(&mut self, path: &str) -> Response {
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
                default_group: "default_group".to_string(),
                ratelimit: None,
                timeout: Some(TimeoutConfig::default()),
                routing: None,
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
    let app_router = v1::api_routes(shared_config.clone());
    println!("API routes created");

    // 返回 TestApp 实例
    TestApp {
        router: app_router,
        config: shared_config,
    }
}
