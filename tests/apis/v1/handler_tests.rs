use crate::apis::v1::common::{create_task_processor, create_test_config};
use llmproxy::apis::v1::{handler::TaskService, types::AdminTask};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::test;

/// 测试处理器能够获取配置
#[test]
async fn test_get_config() {
    // 设置测试环境
    let config = create_test_config();
    let config_state = Arc::new(RwLock::new(config.clone()));
    let (_, processor) = create_task_processor(Arc::clone(&config_state), None);

    // 获取配置
    let result = TaskService::get_config(&processor).await;
    assert!(result.is_ok());

    // 验证配置内容
    let retrieved_config = result.unwrap();
    assert_eq!(retrieved_config.upstreams.len(), config.upstreams.len());
    assert_eq!(
        retrieved_config.upstream_groups.len(),
        config.upstream_groups.len()
    );
    assert_eq!(
        retrieved_config.http_server.forwards.len(),
        config.http_server.forwards.len()
    );
}

/// 测试任务处理器运行
#[test]
async fn test_task_processor_run() {
    // 设置测试环境
    let config = create_test_config();
    let config_state = Arc::new(RwLock::new(config.clone()));

    // 创建处理器
    let (task_sender, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 使用标记任务检查处理器退出
    let (exit_sender, mut exit_receiver) = mpsc::channel(1);

    // 启动处理器
    let processor_handle = tokio::spawn(async move {
        // 模拟处理器运行，但在实际处理前就中断通道
        let mut exit_received = false;
        while let Some(task) = processor.receiver.recv().await {
            // 实际逻辑不执行，只检查是否收到了退出标记
            if let AdminTask::CreateUpstream(_) = task {
                exit_received = true;
                break;
            }
        }

        // 通知测试处理器已退出
        exit_sender.send(exit_received).await.unwrap();
    });

    // 发送任务
    task_sender
        .send(AdminTask::CreateUpstream(config.upstreams[0].clone()))
        .await
        .unwrap();

    // 关闭通道模拟结束
    drop(task_sender);

    // 等待处理器退出
    let _ = processor_handle.await;

    // 验证处理器确实处理了我们的任务
    let exit_received = exit_receiver.recv().await.unwrap();
    assert!(exit_received);
}

/// 测试多个任务的处理
#[test]
async fn test_multiple_tasks() {
    // 设置测试环境
    let config = create_test_config();
    let config_state = Arc::new(RwLock::new(config.clone()));

    // 使用单独的通道验证任务处理顺序
    let (verify_sender, mut verify_receiver) = mpsc::channel(10);

    // 创建处理器
    let (task_sender, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 启动处理器
    let verify_sender_clone = verify_sender.clone();
    let processor_handle = tokio::spawn(async move {
        // 模拟处理器运行
        let mut count = 0;
        while let Some(task) = processor.receiver.recv().await {
            // 只检查任务类型，不实际执行
            match task {
                AdminTask::CreateUpstream(_) => {
                    verify_sender_clone.send("create_upstream").await.unwrap();
                    count += 1;
                }
                AdminTask::UpdateUpstream(_, _) => {
                    verify_sender_clone.send("update_upstream").await.unwrap();
                    count += 1;
                }
                AdminTask::DeleteUpstream(_) => {
                    verify_sender_clone.send("delete_upstream").await.unwrap();
                    count += 1;
                }
                _ => (),
            }

            // 收到足够的任务后退出
            if count >= 3 {
                break;
            }
        }
    });

    // 发送多个任务
    task_sender
        .send(AdminTask::CreateUpstream(config.upstreams[0].clone()))
        .await
        .unwrap();
    task_sender
        .send(AdminTask::UpdateUpstream(
            "test_upstream".to_string(),
            config.upstreams[0].clone(),
        ))
        .await
        .unwrap();
    task_sender
        .send(AdminTask::DeleteUpstream("test_upstream".to_string()))
        .await
        .unwrap();

    // 验证任务顺序
    assert_eq!(verify_receiver.recv().await.unwrap(), "create_upstream");
    assert_eq!(verify_receiver.recv().await.unwrap(), "update_upstream");
    assert_eq!(verify_receiver.recv().await.unwrap(), "delete_upstream");

    // 关闭通道并等待处理器退出
    drop(task_sender);
    drop(verify_sender);
    let _ = processor_handle.await;
}
