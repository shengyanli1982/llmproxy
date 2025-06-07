use crate::apis::v1::{
    forward::forward_routes,
    handler::TaskService,
    types::{ConfigState, ServerManagerSender},
    upstream::upstream_routes,
    upstream_group::upstream_group_routes,
};
use axum::Router;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

// 创建管理 API 路由
pub fn create_admin_api_router(
    config: ConfigState,
    server_manager_sender: ServerManagerSender,
) -> Router {
    // 创建任务通道
    let (sender, receiver) = mpsc::channel(100);

    // 创建任务处理器
    let processor = crate::apis::v1::types::TaskProcessor {
        receiver,
        config: Arc::clone(&config),
    };

    // 启动任务处理器
    let _task_handle: JoinHandle<()> = tokio::spawn(async move {
        let mut processor = processor;
        TaskService::run(&mut processor).await;
    });

    // 创建 API 路由
    Router::new()
        .merge(upstream_routes(Arc::clone(&config), sender.clone()))
        .merge(upstream_group_routes(Arc::clone(&config), sender.clone()))
        .merge(forward_routes(
            Arc::clone(&config),
            sender,
            server_manager_sender,
        ))
}
