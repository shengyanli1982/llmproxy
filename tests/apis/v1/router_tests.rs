use crate::apis::v1::common::create_test_config;
use llmproxy::apis::v1::{router::create_admin_api_router, types::ServerManagerTask};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// 测试路由器创建
#[tokio::test]
async fn test_router_creation() {
    // 设置测试环境
    let config = create_test_config();
    let config_state = Arc::new(RwLock::new(config));
    let (server_sender, _) = mpsc::channel::<ServerManagerTask>(100);

    // 创建路由器
    let router = create_admin_api_router(Arc::clone(&config_state), server_sender);

    // 验证路由器创建成功 - 我们无法直接测试into_make_service的返回值
    // 验证存在路由
    assert!(router.has_routes());
}
