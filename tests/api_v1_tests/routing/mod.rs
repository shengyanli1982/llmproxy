//! 路由规则 API 测试模块

// 导入子模块
pub mod basic_tests;
pub mod error_path_tests;
pub mod extreme_case_tests;
pub mod route_types_tests;

use super::helpers::spawn_app;
use axum::{body::to_bytes, http::StatusCode};
use base64::{engine::general_purpose::URL_SAFE, Engine};
use llmproxy::{
    api::v1::models::{ErrorResponse, SuccessResponse},
    config::{http_server::RoutingRule, UpstreamGroupConfig, UpstreamRef},
};
use serde_json::json;

/// 将路径编码为 base64（URL 安全格式）
pub fn encode_path_to_base64(path: &str) -> String {
    URL_SAFE.encode(path)
}

/// 设置测试所需的上游组
pub async fn setup_test_upstream_groups(app: &mut super::helpers::TestApp) {
    // 添加测试上游组
    let mut config = app.config.write().await;

    // 确保测试组存在
    if !config
        .upstream_groups
        .iter()
        .any(|g| g.name == "test_group")
    {
        config.upstream_groups.push(UpstreamGroupConfig {
            name: "test_group".to_string(),
            upstreams: vec![UpstreamRef {
                name: "default_upstream".to_string(),
                weight: 100,
            }],
            balance: llmproxy::config::BalanceConfig::default(),
            http_client: llmproxy::config::HttpClientConfig::default(),
        });
    }

    // 确保另一个测试组存在（用于更新测试）
    if !config
        .upstream_groups
        .iter()
        .any(|g| g.name == "another_group")
    {
        config.upstream_groups.push(UpstreamGroupConfig {
            name: "another_group".to_string(),
            upstreams: vec![UpstreamRef {
                name: "default_upstream".to_string(),
                weight: 100,
            }],
            balance: llmproxy::config::BalanceConfig::default(),
            http_client: llmproxy::config::HttpClientConfig::default(),
        });
    }
}

/// 添加测试路由规则到指定的转发服务
pub async fn add_test_route(
    app: &mut super::helpers::TestApp,
    forward_name: &str,
    path: &str,
    target_group: &str,
) -> bool {
    let mut config = app.config.write().await;

    // 找到指定的转发服务
    if let Some(server) = config.http_server.as_mut() {
        if let Some(forward) = server.forwards.iter_mut().find(|f| f.name == forward_name) {
            // 初始化 routing 字段（如果不存在）
            if forward.routing.is_none() {
                forward.routing = Some(Vec::new());
            }

            let routing = forward.routing.as_mut().unwrap();

            // 检查路径是否已存在
            if routing.iter().any(|r| r.path == path) {
                return false; // 路径已存在
            }

            // 添加路由规则
            routing.push(RoutingRule {
                path: path.to_string(),
                target_group: target_group.to_string(),
            });

            return true;
        }
    }

    false
}
