use crate::apis::v1::{
    forward::forward_routes,
    handler::TaskService,
    types::{ConfigState, ServerManagerSender, TaskProcessor},
    upstream::upstream_routes,
    upstream_group::upstream_group_routes,
};
use crate::manager::SERVER_MANAGER_TASK_CHANNEL_SIZE;
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
    let (sender, receiver) = mpsc::channel(SERVER_MANAGER_TASK_CHANNEL_SIZE);

    // 创建任务处理器
    let processor = TaskProcessor {
        receiver,
        config: Arc::clone(&config),
        sender: Some(server_manager_sender.clone()),
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
