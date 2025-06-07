use crate::apis::v1::common::{
    create_server_manager_sender, create_task_processor, process_single_task, setup_test_env,
};
use llmproxy::{
    apis::v1::{error::ApiError, types::AdminTask},
    config::{ForwardConfig, RateLimitConfig, TimeoutConfig},
};
use std::sync::Arc;
use tokio::test;

/// 测试创建转发规则
#[test]
async fn test_create_forward() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();

    // 创建服务器管理器通道，确保它是新创建的
    let (server_sender, _server_receiver) = create_server_manager_sender();

    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), Some(server_sender));

    // 创建新的转发规则配置
    let new_forward = ForwardConfig {
        name: "new_forward".to_string(),
        port: 3001,
        address: "127.0.0.1".to_string(),
        upstream_group: "test_group".to_string(),
        ratelimit: RateLimitConfig::default(),
        timeout: TimeoutConfig { connect: 5 },
    };

    // 处理创建任务
    let result = process_single_task(
        &mut processor,
        AdminTask::CreateForward(new_forward.clone()),
    )
    .await;
    assert!(result.is_ok());

    // 验证是否已创建
    let config = config_state.read().await;
    assert_eq!(config.http_server.forwards.len(), 2);
    assert!(config
        .http_server
        .forwards
        .iter()
        .any(|f| f.name == "new_forward"));
}

/// 测试创建已存在的转发规则（名称冲突）
#[test]
async fn test_create_duplicate_forward() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 创建与现有名称相同的转发规则
    let duplicate_forward = ForwardConfig {
        name: "test_forward".to_string(), // 已存在的名称
        port: 3001,
        address: "127.0.0.1".to_string(),
        upstream_group: "test_group".to_string(),
        ratelimit: RateLimitConfig::default(),
        timeout: TimeoutConfig { connect: 5 },
    };

    // 处理创建任务，应该失败
    let result =
        process_single_task(&mut processor, AdminTask::CreateForward(duplicate_forward)).await;
    assert!(result.is_err());

    // 验证错误类型
    match result {
        Err(ApiError::AlreadyExists(_)) => { /* 预期错误 */ }
        _ => panic!("Expected AlreadyExists error, got {:?}", result),
    }
}

/// 测试创建引用不存在上游组的转发规则
#[test]
async fn test_create_forward_with_nonexistent_upstream_group() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(
        Arc::clone(&config_state),
        Some(create_server_manager_sender().0),
    );

    // 创建引用不存在上游组的转发规则
    let invalid_forward = ForwardConfig {
        name: "invalid_forward".to_string(),
        port: 3001,
        address: "127.0.0.1".to_string(),
        upstream_group: "nonexistent_group".to_string(), // 不存在的上游组
        ratelimit: RateLimitConfig::default(),
        timeout: TimeoutConfig { connect: 5 },
    };

    // 处理创建任务，应该失败
    let result =
        process_single_task(&mut processor, AdminTask::CreateForward(invalid_forward)).await;
    assert!(result.is_err());

    // 验证错误类型
    match result {
        Err(ApiError::ReferenceNotFound { .. }) => { /* 预期错误 */ }
        _ => panic!("Expected ReferenceNotFound error, got {:?}", result),
    }
}

/// 测试更新转发规则
#[test]
async fn test_update_forward() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();

    // 创建服务器管理器通道，确保它是新创建的
    let (server_sender, _server_receiver) = create_server_manager_sender();

    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), Some(server_sender));

    // 更新转发规则配置
    let updated_forward = ForwardConfig {
        name: "test_forward".to_string(),
        port: 3001, // 新端口
        address: "127.0.0.1".to_string(),
        upstream_group: "test_group".to_string(),
        ratelimit: RateLimitConfig {
            enabled: true, // 开启限流
            per_second: 50,
            burst: 100,
        },
        timeout: TimeoutConfig { connect: 10 }, // 新超时
    };

    // 处理更新任务
    let result = process_single_task(
        &mut processor,
        AdminTask::UpdateForward("test_forward".to_string(), updated_forward),
    )
    .await;
    assert!(result.is_ok());

    // 验证是否已更新
    let config = config_state.read().await;
    assert_eq!(config.http_server.forwards.len(), 1);
    assert_eq!(config.http_server.forwards[0].port, 3001);
    assert_eq!(config.http_server.forwards[0].timeout.connect, 10);
    assert!(config.http_server.forwards[0].ratelimit.enabled);
}

/// 测试更新不改变监听地址的转发规则
#[test]
async fn test_update_forward_without_address_change() {
    // 设置测试环境
    let (config_state, _, mut server_receiver) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 更新转发规则配置（不改变地址和端口）
    let updated_forward = ForwardConfig {
        name: "test_forward".to_string(),
        port: 3000,                       // 保持原端口
        address: "127.0.0.1".to_string(), // 保持原地址
        upstream_group: "test_group".to_string(),
        ratelimit: RateLimitConfig {
            enabled: true, // 开启限流
            per_second: 50,
            burst: 100,
        },
        timeout: TimeoutConfig { connect: 10 }, // 更新超时
    };

    // 处理更新任务
    let result = process_single_task(
        &mut processor,
        AdminTask::UpdateForward("test_forward".to_string(), updated_forward),
    )
    .await;
    assert!(result.is_ok());

    // 验证是否已更新
    let config = config_state.read().await;
    assert_eq!(config.http_server.forwards[0].port, 3000);
    assert_eq!(config.http_server.forwards[0].timeout.connect, 10);
    assert!(config.http_server.forwards[0].ratelimit.enabled);

    // 验证不应该收到任何服务器管理任务
    if let Ok(Some(_)) = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        server_receiver.recv(),
    )
    .await
    {
        panic!("Received unexpected server manager task");
    }
}

/// 测试更新不存在的转发规则
#[test]
async fn test_update_nonexistent_forward() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 更新不存在的转发规则
    let updated_forward = ForwardConfig {
        name: "nonexistent_forward".to_string(),
        port: 3001,
        address: "127.0.0.1".to_string(),
        upstream_group: "test_group".to_string(),
        ratelimit: RateLimitConfig::default(),
        timeout: TimeoutConfig { connect: 5 },
    };

    // 处理更新任务，应该失败
    let result = process_single_task(
        &mut processor,
        AdminTask::UpdateForward("nonexistent_forward".to_string(), updated_forward),
    )
    .await;
    assert!(result.is_err());

    // 验证错误类型
    match result {
        Err(ApiError::NotFound(_)) => { /* 预期错误 */ }
        _ => panic!("Expected NotFound error, got {:?}", result),
    }
}

/// 测试更新转发规则时名称不匹配
#[test]
async fn test_update_forward_name_mismatch() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 更新转发规则，但名称与URL不匹配
    let mismatched_forward = ForwardConfig {
        name: "different_name".to_string(), // 与URL参数不匹配
        port: 3001,
        address: "127.0.0.1".to_string(),
        upstream_group: "test_group".to_string(),
        ratelimit: RateLimitConfig::default(),
        timeout: TimeoutConfig { connect: 5 },
    };

    // 处理更新任务，应该失败
    let result = process_single_task(
        &mut processor,
        AdminTask::UpdateForward("test_forward".to_string(), mismatched_forward),
    )
    .await;
    assert!(result.is_err());

    // 验证错误类型
    match result {
        Err(ApiError::ValidationError(_)) => { /* 预期错误 */ }
        _ => panic!("Expected ValidationError error, got {:?}", result),
    }
}

/// 测试更新转发规则引用不存在的上游组
#[test]
async fn test_update_forward_with_nonexistent_upstream_group() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(
        Arc::clone(&config_state),
        Some(create_server_manager_sender().0),
    );

    // 更新转发规则，引用不存在的上游组
    let invalid_forward = ForwardConfig {
        name: "test_forward".to_string(),
        port: 3001,
        address: "127.0.0.1".to_string(),
        upstream_group: "nonexistent_group".to_string(), // 不存在的上游组
        ratelimit: RateLimitConfig::default(),
        timeout: TimeoutConfig { connect: 5 },
    };

    // 处理更新任务，应该失败
    let result = process_single_task(
        &mut processor,
        AdminTask::UpdateForward("test_forward".to_string(), invalid_forward),
    )
    .await;
    assert!(result.is_err());

    // 验证错误类型
    match result {
        Err(ApiError::ReferenceNotFound { .. }) => { /* 预期错误 */ }
        _ => panic!("Expected ReferenceNotFound error, got {:?}", result),
    }
}

/// 测试删除转发规则
#[test]
async fn test_delete_forward() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();

    // 创建服务器管理器通道，确保它是新创建的
    let (server_sender, _server_receiver) = create_server_manager_sender();

    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), Some(server_sender));

    // 处理删除任务
    let result = process_single_task(
        &mut processor,
        AdminTask::DeleteForward("test_forward".to_string()),
    )
    .await;
    assert!(result.is_ok());

    // 验证是否已删除
    let config = config_state.read().await;
    assert_eq!(config.http_server.forwards.len(), 0);
}

/// 测试删除不存在的转发规则
#[test]
async fn test_delete_nonexistent_forward() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 删除不存在的转发规则
    let result = process_single_task(
        &mut processor,
        AdminTask::DeleteForward("nonexistent_forward".to_string()),
    )
    .await;

    // 验证操作成功（幂等性）
    assert!(result.is_ok());
}

/// 测试删除转发规则的幂等性
#[test]
async fn test_delete_forward_idempotent() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();

    // 先创建一个新的转发规则
    {
        let (server_sender, _) = create_server_manager_sender();
        let (_, mut processor) =
            create_task_processor(Arc::clone(&config_state), Some(server_sender));

        let new_forward = ForwardConfig {
            name: "forward_to_delete".to_string(),
            port: 3002,
            address: "127.0.0.1".to_string(),
            upstream_group: "test_group".to_string(),
            ratelimit: RateLimitConfig::default(),
            timeout: TimeoutConfig { connect: 5 },
        };

        let result =
            process_single_task(&mut processor, AdminTask::CreateForward(new_forward)).await;
        assert!(result.is_ok());
    }

    // 第一次删除
    {
        let (server_sender, _) = create_server_manager_sender();
        let (_, mut processor) =
            create_task_processor(Arc::clone(&config_state), Some(server_sender));

        let result = process_single_task(
            &mut processor,
            AdminTask::DeleteForward("forward_to_delete".to_string()),
        )
        .await;
        assert!(result.is_ok());
    }

    // 第二次删除（测试幂等性）
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result = process_single_task(
            &mut processor,
            AdminTask::DeleteForward("forward_to_delete".to_string()),
        )
        .await;
        assert!(result.is_ok(), "第二次删除应该成功，体现幂等性");
    }

    // 验证转发规则确实已被删除
    let config = config_state.read().await;
    assert!(!config
        .http_server
        .forwards
        .iter()
        .any(|f| f.name == "forward_to_delete"));
}
