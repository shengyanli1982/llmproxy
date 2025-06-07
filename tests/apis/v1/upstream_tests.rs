use crate::apis::v1::common::{create_task_processor, process_single_task, setup_test_env};
use llmproxy::{
    apis::v1::{error::ApiError, types::AdminTask},
    config::UpstreamConfig,
};
use std::sync::Arc;
use tokio::test;
use uuid::Uuid;

/// 测试创建上游服务
#[test]
async fn test_create_upstream() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 创建新的上游配置
    let new_upstream = UpstreamConfig {
        name: "new_upstream".to_string(),
        url: "http://localhost:9090".to_string(),
        id: Uuid::new_v4().to_string(),
        auth: None,
        headers: vec![],
        breaker: None,
    };

    // 处理创建任务
    let result = process_single_task(
        &mut processor,
        AdminTask::CreateUpstream(new_upstream.clone()),
    )
    .await;
    assert!(result.is_ok());

    // 验证是否已创建
    let config = config_state.read().await;
    assert_eq!(config.upstreams.len(), 2);
    assert!(config.upstreams.iter().any(|u| u.name == "new_upstream"));
}

/// 测试创建已存在的上游服务（重名冲突）
#[test]
async fn test_create_duplicate_upstream() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 创建与现有名称相同的上游配置
    let duplicate_upstream = UpstreamConfig {
        name: "test_upstream".to_string(), // 已存在的名称
        url: "http://localhost:9090".to_string(),
        id: Uuid::new_v4().to_string(),
        auth: None,
        headers: vec![],
        breaker: None,
    };

    // 处理创建任务，应该失败
    let result = process_single_task(
        &mut processor,
        AdminTask::CreateUpstream(duplicate_upstream),
    )
    .await;
    assert!(result.is_err());

    // 验证错误类型
    match result {
        Err(ApiError::AlreadyExists(_)) => { /* 预期错误 */ }
        _ => panic!("Expected AlreadyExists error, got {:?}", result),
    }
}

/// 测试创建无效的上游服务（验证失败）
#[test]
async fn test_create_invalid_upstream() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 创建无效的上游配置（无效URL）
    let invalid_upstream = UpstreamConfig {
        name: "invalid_upstream".to_string(),
        url: "invalid-url".to_string(), // 无效URL
        id: Uuid::new_v4().to_string(),
        auth: None,
        headers: vec![],
        breaker: None,
    };

    // 处理创建任务，应该失败
    let result =
        process_single_task(&mut processor, AdminTask::CreateUpstream(invalid_upstream)).await;
    assert!(result.is_err());

    // 验证错误类型
    match result {
        Err(ApiError::ValidationError(_)) => { /* 预期错误 */ }
        _ => panic!("Expected ValidationError error, got {:?}", result),
    }
}

/// 测试更新上游服务
#[test]
async fn test_update_upstream() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 获取原始ID
    let original_id = {
        let config = config_state.read().await;
        config.upstreams[0].id.clone()
    };

    // 更新上游配置
    let updated_upstream = UpstreamConfig {
        name: "test_upstream".to_string(),
        url: "http://localhost:9999".to_string(), // 新URL
        id: "".to_string(),                       // ID应该被保留
        auth: None,
        headers: vec![],
        breaker: None,
    };

    // 处理更新任务
    let result = process_single_task(
        &mut processor,
        AdminTask::UpdateUpstream("test_upstream".to_string(), updated_upstream),
    )
    .await;
    assert!(result.is_ok());

    // 验证是否已更新
    let config = config_state.read().await;
    assert_eq!(config.upstreams.len(), 1);
    assert_eq!(config.upstreams[0].url, "http://localhost:9999");
    // 确认ID被保留
    assert_eq!(config.upstreams[0].id, original_id);
}

/// 测试更新不存在的上游服务
#[test]
async fn test_update_nonexistent_upstream() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 更新不存在的上游配置
    let updated_upstream = UpstreamConfig {
        name: "nonexistent_upstream".to_string(),
        url: "http://localhost:9999".to_string(),
        id: Uuid::new_v4().to_string(),
        auth: None,
        headers: vec![],
        breaker: None,
    };

    // 处理更新任务，应该失败
    let result = process_single_task(
        &mut processor,
        AdminTask::UpdateUpstream("nonexistent_upstream".to_string(), updated_upstream),
    )
    .await;
    assert!(result.is_err());

    // 验证错误类型
    match result {
        Err(ApiError::NotFound(_)) => { /* 预期错误 */ }
        _ => panic!("Expected NotFound error, got {:?}", result),
    }
}

/// 测试名称不匹配的上游服务更新
#[test]
async fn test_update_upstream_name_mismatch() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 更新配置，名称与路径不匹配
    let updated_upstream = UpstreamConfig {
        name: "different_name".to_string(), // 与路径参数不一致
        url: "http://localhost:9999".to_string(),
        id: Uuid::new_v4().to_string(),
        auth: None,
        headers: vec![],
        breaker: None,
    };

    // 处理更新任务，应该失败
    let result = process_single_task(
        &mut processor,
        AdminTask::UpdateUpstream("test_upstream".to_string(), updated_upstream),
    )
    .await;
    assert!(result.is_err());

    // 验证错误类型
    match result {
        Err(ApiError::ValidationError(_)) => { /* 预期错误 */ }
        _ => panic!("Expected ValidationError error, got {:?}", result),
    }
}

/// 测试删除上游服务
#[test]
async fn test_delete_upstream() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();

    // 先创建一个不被引用的上游
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let new_upstream = UpstreamConfig {
            name: "unused_upstream".to_string(),
            url: "http://localhost:9090".to_string(),
            id: Uuid::new_v4().to_string(),
            auth: None,
            headers: vec![],
            breaker: None,
        };

        let result =
            process_single_task(&mut processor, AdminTask::CreateUpstream(new_upstream)).await;
        assert!(result.is_ok());
    }

    // 现在删除这个不被引用的上游
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result = process_single_task(
            &mut processor,
            AdminTask::DeleteUpstream("unused_upstream".to_string()),
        )
        .await;
        assert!(result.is_ok());
    }

    // 验证是否已删除
    let config = config_state.read().await;
    assert_eq!(config.upstreams.len(), 1); // 应该只剩下被引用的上游
    assert!(!config.upstreams.iter().any(|u| u.name == "unused_upstream"));
}

/// 测试删除被引用的上游服务
#[test]
async fn test_delete_referenced_upstream() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 尝试删除被上游组引用的上游
    let result = process_single_task(
        &mut processor,
        AdminTask::DeleteUpstream("test_upstream".to_string()),
    )
    .await;
    assert!(result.is_err());

    // 验证错误类型
    match result {
        Err(ApiError::StillReferenced { .. }) => { /* 预期错误 */ }
        _ => panic!("Expected StillReferenced error, got {:?}", result),
    }

    // 验证上游是否仍然存在
    let config = config_state.read().await;
    assert!(config.upstreams.iter().any(|u| u.name == "test_upstream"));
}

/// 测试删除不存在的上游服务（幂等性测试）
#[test]
async fn test_delete_nonexistent_upstream() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 尝试删除不存在的上游
    let result = process_single_task(
        &mut processor,
        AdminTask::DeleteUpstream("nonexistent_upstream".to_string()),
    )
    .await;

    // 应该成功（幂等性）
    assert!(result.is_ok());
}
