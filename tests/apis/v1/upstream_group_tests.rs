use crate::apis::v1::common::{create_task_processor, process_single_task, setup_test_env};
use llmproxy::{
    apis::v1::{error::ApiError, types::AdminTask},
    config::{BalanceConfig, BalanceStrategy, HttpClientConfig, UpstreamGroupConfig, UpstreamRef},
};
use std::sync::Arc;
use tokio::test;

/// 测试创建上游组
#[test]
async fn test_create_upstream_group() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 创建新的上游组配置
    let new_group = UpstreamGroupConfig {
        name: "new_group".to_string(),
        upstreams: vec![UpstreamRef {
            name: "test_upstream".to_string(),
            weight: 1,
        }],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: HttpClientConfig::default(),
    };

    // 处理创建任务
    let result = process_single_task(
        &mut processor,
        AdminTask::CreateUpstreamGroup(new_group.clone()),
    )
    .await;
    assert!(result.is_ok());

    // 验证是否已创建
    let config = config_state.read().await;
    assert_eq!(config.upstream_groups.len(), 2);
    assert!(config.upstream_groups.iter().any(|g| g.name == "new_group"));
}

/// 测试创建已存在的上游组（名称冲突）
#[test]
async fn test_create_duplicate_upstream_group() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 创建与现有名称相同的上游组
    let duplicate_group = UpstreamGroupConfig {
        name: "test_group".to_string(), // 已存在的名称
        upstreams: vec![UpstreamRef {
            name: "test_upstream".to_string(),
            weight: 1,
        }],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: HttpClientConfig::default(),
    };

    // 处理创建任务，应该失败
    let result = process_single_task(
        &mut processor,
        AdminTask::CreateUpstreamGroup(duplicate_group),
    )
    .await;
    assert!(result.is_err());

    // 验证错误类型
    match result {
        Err(ApiError::AlreadyExists(_)) => { /* 预期错误 */ }
        _ => panic!("Expected AlreadyExists error, got {:?}", result),
    }
}

/// 测试创建引用不存在上游的上游组
#[test]
async fn test_create_upstream_group_with_nonexistent_upstream() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 创建引用不存在上游的上游组
    let invalid_group = UpstreamGroupConfig {
        name: "invalid_group".to_string(),
        upstreams: vec![UpstreamRef {
            name: "nonexistent_upstream".to_string(), // 不存在的上游
            weight: 1,
        }],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: HttpClientConfig::default(),
    };

    // 处理创建任务，应该失败
    let result = process_single_task(
        &mut processor,
        AdminTask::CreateUpstreamGroup(invalid_group),
    )
    .await;
    assert!(result.is_err());

    // 验证错误类型
    match result {
        Err(ApiError::ReferenceNotFound { .. }) => { /* 预期错误 */ }
        _ => panic!("Expected ReferenceNotFound error, got {:?}", result),
    }
}

/// 测试更新上游组
#[test]
async fn test_update_upstream_group() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 更新上游组配置
    let updated_group = UpstreamGroupConfig {
        name: "test_group".to_string(),
        upstreams: vec![UpstreamRef {
            name: "test_upstream".to_string(),
            weight: 5, // 新权重
        }],
        balance: BalanceConfig {
            strategy: BalanceStrategy::WeightedRoundRobin, // 新策略
        },
        http_client: HttpClientConfig::default(),
    };

    // 处理更新任务
    let result = process_single_task(
        &mut processor,
        AdminTask::UpdateUpstreamGroup("test_group".to_string(), updated_group),
    )
    .await;
    assert!(result.is_ok());

    // 验证是否已更新
    let config = config_state.read().await;
    assert_eq!(config.upstream_groups.len(), 1);
    assert_eq!(config.upstream_groups[0].upstreams[0].weight, 5);
    assert!(matches!(
        config.upstream_groups[0].balance.strategy,
        BalanceStrategy::WeightedRoundRobin
    ));
}

/// 测试更新不存在的上游组
#[test]
async fn test_update_nonexistent_upstream_group() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 更新不存在的上游组
    let updated_group = UpstreamGroupConfig {
        name: "nonexistent_group".to_string(),
        upstreams: vec![UpstreamRef {
            name: "test_upstream".to_string(),
            weight: 1,
        }],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: HttpClientConfig::default(),
    };

    // 处理更新任务，应该失败
    let result = process_single_task(
        &mut processor,
        AdminTask::UpdateUpstreamGroup("nonexistent_group".to_string(), updated_group),
    )
    .await;
    assert!(result.is_err());

    // 验证错误类型
    match result {
        Err(ApiError::NotFound(_)) => { /* 预期错误 */ }
        _ => panic!("Expected NotFound error, got {:?}", result),
    }
}

/// 测试名称不匹配的上游组更新
#[test]
async fn test_update_upstream_group_name_mismatch() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 更新配置，名称与路径不匹配
    let updated_group = UpstreamGroupConfig {
        name: "different_name".to_string(), // 与路径参数不一致
        upstreams: vec![UpstreamRef {
            name: "test_upstream".to_string(),
            weight: 1,
        }],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: HttpClientConfig::default(),
    };

    // 处理更新任务，应该失败
    let result = process_single_task(
        &mut processor,
        AdminTask::UpdateUpstreamGroup("test_group".to_string(), updated_group),
    )
    .await;
    assert!(result.is_err());

    // 验证错误类型
    match result {
        Err(ApiError::ValidationError(_)) => { /* 预期错误 */ }
        _ => panic!("Expected ValidationError error, got {:?}", result),
    }
}

/// 测试更新上游组时引用不存在的上游
#[test]
async fn test_update_upstream_group_with_nonexistent_upstream() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 更新上游组，引用不存在的上游
    let updated_group = UpstreamGroupConfig {
        name: "test_group".to_string(),
        upstreams: vec![UpstreamRef {
            name: "nonexistent_upstream".to_string(), // 不存在的上游
            weight: 1,
        }],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: HttpClientConfig::default(),
    };

    // 处理更新任务，应该失败
    let result = process_single_task(
        &mut processor,
        AdminTask::UpdateUpstreamGroup("test_group".to_string(), updated_group),
    )
    .await;
    assert!(result.is_err());

    // 验证错误类型
    match result {
        Err(ApiError::ReferenceNotFound { .. }) => { /* 预期错误 */ }
        _ => panic!("Expected ReferenceNotFound error, got {:?}", result),
    }
}

/// 测试删除上游组
#[test]
async fn test_delete_upstream_group() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();

    // 先创建一个不被引用的上游组
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let new_group = UpstreamGroupConfig {
            name: "unused_group".to_string(),
            upstreams: vec![UpstreamRef {
                name: "test_upstream".to_string(),
                weight: 1,
            }],
            balance: BalanceConfig {
                strategy: BalanceStrategy::RoundRobin,
            },
            http_client: HttpClientConfig::default(),
        };

        let result =
            process_single_task(&mut processor, AdminTask::CreateUpstreamGroup(new_group)).await;
        assert!(result.is_ok());
    }

    // 现在删除这个不被引用的上游组
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result = process_single_task(
            &mut processor,
            AdminTask::DeleteUpstreamGroup("unused_group".to_string()),
        )
        .await;
        assert!(result.is_ok());
    }

    // 验证是否已删除
    let config = config_state.read().await;
    assert_eq!(config.upstream_groups.len(), 1); // 应该只剩下被引用的上游组
    assert!(!config
        .upstream_groups
        .iter()
        .any(|g| g.name == "unused_group"));
}

/// 测试删除被引用的上游组
#[test]
async fn test_delete_referenced_upstream_group() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 尝试删除被转发规则引用的上游组
    let result = process_single_task(
        &mut processor,
        AdminTask::DeleteUpstreamGroup("test_group".to_string()),
    )
    .await;
    assert!(result.is_err());

    // 验证错误类型
    match result {
        Err(ApiError::StillReferenced { .. }) => { /* 预期错误 */ }
        _ => panic!("Expected StillReferenced error, got {:?}", result),
    }

    // 验证上游组仍然存在
    let config = config_state.read().await;
    assert!(config
        .upstream_groups
        .iter()
        .any(|g| g.name == "test_group"));
}

/// 测试删除不存在的上游组（幂等性测试）
#[test]
async fn test_delete_nonexistent_upstream_group() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();
    let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);

    // 尝试删除不存在的上游组
    let result = process_single_task(
        &mut processor,
        AdminTask::DeleteUpstreamGroup("nonexistent_group".to_string()),
    )
    .await;

    // 应该成功（幂等性）
    assert!(result.is_ok());
}
