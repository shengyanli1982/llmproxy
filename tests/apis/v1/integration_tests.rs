use crate::apis::v1::common::{create_task_processor, process_single_task, setup_test_env};
use llmproxy::{
    apis::v1::{error::ApiError, types::AdminTask},
    config::{
        BalanceConfig, BalanceStrategy, ForwardConfig, HttpClientConfig, RateLimitConfig,
        TimeoutConfig, UpstreamConfig, UpstreamGroupConfig, UpstreamRef,
    },
};
use std::sync::Arc;
use uuid::Uuid;

// 测试完整的资源创建流程
#[tokio::test]
async fn test_complete_resource_lifecycle() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();

    // 1. 创建一个新的上游
    let upstream_name = "lifecycle_upstream";
    let upstream = UpstreamConfig {
        name: upstream_name.to_string(),
        url: "http://lifecycle-test:8080".to_string(),
        id: Uuid::new_v4().to_string(),
        auth: None,
        headers: vec![],
        breaker: None,
    };

    // 创建上游
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result =
            process_single_task(&mut processor, AdminTask::CreateUpstream(upstream.clone())).await;
        assert!(result.is_ok(), "创建上游失败: {:?}", result);
    }

    // 2. 创建一个引用该上游的上游组
    let group_name = "lifecycle_group";
    let group = UpstreamGroupConfig {
        name: group_name.to_string(),
        upstreams: vec![UpstreamRef {
            name: upstream_name.to_string(),
            weight: 1,
        }],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: HttpClientConfig::default(),
    };

    // 创建上游组
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result = process_single_task(
            &mut processor,
            AdminTask::CreateUpstreamGroup(group.clone()),
        )
        .await;
        assert!(result.is_ok(), "创建上游组失败: {:?}", result);
    }

    // 3. 创建一个引用该上游组的转发规则
    let forward_name = "lifecycle_forward";
    let forward = ForwardConfig {
        name: forward_name.to_string(),
        port: 4000,
        address: "127.0.0.1".to_string(),
        upstream_group: group_name.to_string(),
        ratelimit: RateLimitConfig::default(),
        timeout: TimeoutConfig { connect: 5 },
    };

    // 创建转发规则
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result =
            process_single_task(&mut processor, AdminTask::CreateForward(forward.clone())).await;
        assert!(result.is_ok(), "创建转发规则失败: {:?}", result);
    }

    // 4. 验证所有资源都已创建
    {
        let config = config_state.read().await;
        assert!(
            config.upstreams.iter().any(|u| u.name == upstream_name),
            "上游未创建成功"
        );
        assert!(
            config.upstream_groups.iter().any(|g| g.name == group_name),
            "上游组未创建成功"
        );
        assert!(
            config
                .http_server
                .forwards
                .iter()
                .any(|f| f.name == forward_name),
            "转发规则未创建成功"
        );
    }

    // 5. 尝试删除上游（应该失败，因为被上游组引用）
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result = process_single_task(
            &mut processor,
            AdminTask::DeleteUpstream(upstream_name.to_string()),
        )
        .await;

        assert!(result.is_err(), "删除被引用的上游应该失败");
        match result {
            Err(ApiError::StillReferenced {
                resource_type,
                name,
                ..
            }) => {
                assert_eq!(resource_type, "Upstream");
                assert_eq!(name, upstream_name);
            }
            _ => panic!("预期 StillReferenced 错误，但得到: {:?}", result),
        }
    }

    // 6. 尝试删除上游组（应该失败，因为被转发规则引用）
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result = process_single_task(
            &mut processor,
            AdminTask::DeleteUpstreamGroup(group_name.to_string()),
        )
        .await;

        assert!(result.is_err(), "删除被引用的上游组应该失败");
        match result {
            Err(ApiError::StillReferenced {
                resource_type,
                name,
                ..
            }) => {
                assert_eq!(resource_type, "UpstreamGroup");
                assert_eq!(name, group_name);
            }
            _ => panic!("预期 StillReferenced 错误，但得到: {:?}", result),
        }
    }

    // 7. 删除转发规则
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result = process_single_task(
            &mut processor,
            AdminTask::DeleteForward(forward_name.to_string()),
        )
        .await;

        assert!(result.is_ok(), "删除转发规则失败: {:?}", result);
    }

    // 8. 现在可以删除上游组
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result = process_single_task(
            &mut processor,
            AdminTask::DeleteUpstreamGroup(group_name.to_string()),
        )
        .await;

        assert!(result.is_ok(), "删除上游组失败: {:?}", result);
    }

    // 9. 最后可以删除上游
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result = process_single_task(
            &mut processor,
            AdminTask::DeleteUpstream(upstream_name.to_string()),
        )
        .await;

        assert!(result.is_ok(), "删除上游失败: {:?}", result);
    }

    // 10. 验证所有资源都已删除
    {
        let config = config_state.read().await;
        assert!(
            !config.upstreams.iter().any(|u| u.name == upstream_name),
            "上游未删除成功"
        );
        assert!(
            !config.upstream_groups.iter().any(|g| g.name == group_name),
            "上游组未删除成功"
        );
        assert!(
            !config
                .http_server
                .forwards
                .iter()
                .any(|f| f.name == forward_name),
            "转发规则未删除成功"
        );
    }
}

// 测试并发操作
#[tokio::test]
async fn test_concurrent_operations() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();

    // 创建初始资源
    let upstream_name = "concurrent_upstream";
    let group_name = "concurrent_group";

    // 1. 创建上游
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let upstream = UpstreamConfig {
            name: upstream_name.to_string(),
            url: "http://concurrent-test:8080".to_string(),
            id: Uuid::new_v4().to_string(),
            auth: None,
            headers: vec![],
            breaker: None,
        };

        let result = process_single_task(&mut processor, AdminTask::CreateUpstream(upstream)).await;
        assert!(result.is_ok());
    }

    // 2. 创建上游组
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let group = UpstreamGroupConfig {
            name: group_name.to_string(),
            upstreams: vec![UpstreamRef {
                name: upstream_name.to_string(),
                weight: 1,
            }],
            balance: BalanceConfig {
                strategy: BalanceStrategy::RoundRobin,
            },
            http_client: HttpClientConfig::default(),
        };

        let result =
            process_single_task(&mut processor, AdminTask::CreateUpstreamGroup(group)).await;
        assert!(result.is_ok());
    }

    // 3. 并发执行多个操作
    let config_state_clone = Arc::clone(&config_state);

    // 创建多个任务
    let update_upstream = tokio::spawn(async move {
        let (_, mut processor) = create_task_processor(config_state_clone, None);
        let updated_upstream = UpstreamConfig {
            name: upstream_name.to_string(),
            url: "http://updated-concurrent:9090".to_string(),
            id: Uuid::new_v4().to_string(),
            auth: None,
            headers: vec![],
            breaker: None,
        };

        process_single_task(
            &mut processor,
            AdminTask::UpdateUpstream(upstream_name.to_string(), updated_upstream),
        )
        .await
    });

    let config_state_clone = Arc::clone(&config_state);
    let update_group = tokio::spawn(async move {
        let (_, mut processor) = create_task_processor(config_state_clone, None);
        let updated_group = UpstreamGroupConfig {
            name: group_name.to_string(),
            upstreams: vec![UpstreamRef {
                name: upstream_name.to_string(),
                weight: 2, // 修改权重
            }],
            balance: BalanceConfig {
                strategy: BalanceStrategy::RoundRobin,
            },
            http_client: HttpClientConfig::default(),
        };

        process_single_task(
            &mut processor,
            AdminTask::UpdateUpstreamGroup(group_name.to_string(), updated_group),
        )
        .await
    });

    // 等待所有任务完成
    let update_upstream_result = update_upstream.await.unwrap();
    let update_group_result = update_group.await.unwrap();

    // 验证所有操作都成功
    assert!(
        update_upstream_result.is_ok(),
        "更新上游失败: {:?}",
        update_upstream_result
    );
    assert!(
        update_group_result.is_ok(),
        "更新上游组失败: {:?}",
        update_group_result
    );

    // 验证更新后的状态
    {
        let config = config_state.read().await;

        // 验证上游已更新
        let upstream = config
            .upstreams
            .iter()
            .find(|u| u.name == upstream_name)
            .expect("上游不存在");
        assert_eq!(upstream.url, "http://updated-concurrent:9090");

        // 验证上游组已更新
        let group = config
            .upstream_groups
            .iter()
            .find(|g| g.name == group_name)
            .expect("上游组不存在");
        assert_eq!(group.upstreams[0].weight, 2);
    }
}

// 测试级联引用检查
#[tokio::test]
async fn test_cascading_reference_check() {
    // 设置测试环境
    let (config_state, _, _) = setup_test_env();

    // 创建级联引用结构：
    // upstream1 <- group1 <- forward1
    // upstream1 <- group2 <- forward2
    // upstream2 <- group2

    let upstream1_name = "cascade_upstream1";
    let upstream2_name = "cascade_upstream2";
    let group1_name = "cascade_group1";
    let group2_name = "cascade_group2";
    let forward1_name = "cascade_forward1";
    let forward2_name = "cascade_forward2";

    // 1. 创建上游1
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let upstream = UpstreamConfig {
            name: upstream1_name.to_string(),
            url: "http://cascade-test1:8080".to_string(),
            id: Uuid::new_v4().to_string(),
            auth: None,
            headers: vec![],
            breaker: None,
        };

        let result = process_single_task(&mut processor, AdminTask::CreateUpstream(upstream)).await;
        assert!(result.is_ok());
    }

    // 2. 创建上游2
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let upstream = UpstreamConfig {
            name: upstream2_name.to_string(),
            url: "http://cascade-test2:8080".to_string(),
            id: Uuid::new_v4().to_string(),
            auth: None,
            headers: vec![],
            breaker: None,
        };

        let result = process_single_task(&mut processor, AdminTask::CreateUpstream(upstream)).await;
        assert!(result.is_ok());
    }

    // 3. 创建上游组1 (引用上游1)
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let group = UpstreamGroupConfig {
            name: group1_name.to_string(),
            upstreams: vec![UpstreamRef {
                name: upstream1_name.to_string(),
                weight: 1,
            }],
            balance: BalanceConfig {
                strategy: BalanceStrategy::RoundRobin,
            },
            http_client: HttpClientConfig::default(),
        };

        let result =
            process_single_task(&mut processor, AdminTask::CreateUpstreamGroup(group)).await;
        assert!(result.is_ok());
    }

    // 4. 创建上游组2 (引用上游1和上游2)
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let group = UpstreamGroupConfig {
            name: group2_name.to_string(),
            upstreams: vec![
                UpstreamRef {
                    name: upstream1_name.to_string(),
                    weight: 1,
                },
                UpstreamRef {
                    name: upstream2_name.to_string(),
                    weight: 1,
                },
            ],
            balance: BalanceConfig {
                strategy: BalanceStrategy::RoundRobin,
            },
            http_client: HttpClientConfig::default(),
        };

        let result =
            process_single_task(&mut processor, AdminTask::CreateUpstreamGroup(group)).await;
        assert!(result.is_ok());
    }

    // 5. 创建转发规则1 (引用上游组1)
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let forward = ForwardConfig {
            name: forward1_name.to_string(),
            port: 5000,
            address: "127.0.0.1".to_string(),
            upstream_group: group1_name.to_string(),
            ratelimit: RateLimitConfig::default(),
            timeout: TimeoutConfig { connect: 5 },
        };

        let result = process_single_task(&mut processor, AdminTask::CreateForward(forward)).await;
        assert!(result.is_ok());
    }

    // 6. 创建转发规则2 (引用上游组2)
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let forward = ForwardConfig {
            name: forward2_name.to_string(),
            port: 5001,
            address: "127.0.0.1".to_string(),
            upstream_group: group2_name.to_string(),
            ratelimit: RateLimitConfig::default(),
            timeout: TimeoutConfig { connect: 5 },
        };

        let result = process_single_task(&mut processor, AdminTask::CreateForward(forward)).await;
        assert!(result.is_ok());
    }

    // 7. 尝试删除上游1 (应该失败，因为被两个上游组引用)
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result = process_single_task(
            &mut processor,
            AdminTask::DeleteUpstream(upstream1_name.to_string()),
        )
        .await;

        assert!(result.is_err(), "删除被多个上游组引用的上游应该失败");
        match result {
            Err(ApiError::StillReferenced {
                resource_type,
                name,
                referenced_by,
            }) => {
                assert_eq!(resource_type, "Upstream");
                assert_eq!(name, upstream1_name);
                // 验证引用信息包含两个上游组
                assert_eq!(referenced_by.len(), 2);
                assert!(referenced_by.iter().any(|r| r.contains(group1_name)));
                assert!(referenced_by.iter().any(|r| r.contains(group2_name)));
            }
            _ => panic!("预期 StillReferenced 错误，但得到: {:?}", result),
        }
    }

    // 8. 尝试删除上游组1 (应该失败，因为被转发规则1引用)
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result = process_single_task(
            &mut processor,
            AdminTask::DeleteUpstreamGroup(group1_name.to_string()),
        )
        .await;

        assert!(result.is_err(), "删除被转发规则引用的上游组应该失败");
        match result {
            Err(ApiError::StillReferenced {
                resource_type,
                name,
                referenced_by,
            }) => {
                assert_eq!(resource_type, "UpstreamGroup");
                assert_eq!(name, group1_name);
                assert_eq!(referenced_by.len(), 1);
                assert!(referenced_by[0].contains(forward1_name));
            }
            _ => panic!("预期 StillReferenced 错误，但得到: {:?}", result),
        }
    }

    // 9. 删除转发规则1
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result = process_single_task(
            &mut processor,
            AdminTask::DeleteForward(forward1_name.to_string()),
        )
        .await;

        assert!(result.is_ok(), "删除转发规则1失败: {:?}", result);
    }

    // 10. 现在可以删除上游组1
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result = process_single_task(
            &mut processor,
            AdminTask::DeleteUpstreamGroup(group1_name.to_string()),
        )
        .await;

        assert!(result.is_ok(), "删除上游组1失败: {:?}", result);
    }

    // 11. 尝试删除上游1 (仍应该失败，因为还被上游组2引用)
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result = process_single_task(
            &mut processor,
            AdminTask::DeleteUpstream(upstream1_name.to_string()),
        )
        .await;

        assert!(result.is_err(), "删除仍被上游组2引用的上游1应该失败");
        match result {
            Err(ApiError::StillReferenced {
                resource_type,
                name,
                referenced_by,
            }) => {
                assert_eq!(resource_type, "Upstream");
                assert_eq!(name, upstream1_name);
                assert_eq!(referenced_by.len(), 1);
                assert!(referenced_by[0].contains(group2_name));
            }
            _ => panic!("预期 StillReferenced 错误，但得到: {:?}", result),
        }
    }

    // 12. 删除转发规则2
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result = process_single_task(
            &mut processor,
            AdminTask::DeleteForward(forward2_name.to_string()),
        )
        .await;

        assert!(result.is_ok(), "删除转发规则2失败: {:?}", result);
    }

    // 13. 删除上游组2
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result = process_single_task(
            &mut processor,
            AdminTask::DeleteUpstreamGroup(group2_name.to_string()),
        )
        .await;

        assert!(result.is_ok(), "删除上游组2失败: {:?}", result);
    }

    // 14. 现在可以删除上游1和上游2
    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result = process_single_task(
            &mut processor,
            AdminTask::DeleteUpstream(upstream1_name.to_string()),
        )
        .await;

        assert!(result.is_ok(), "删除上游1失败: {:?}", result);
    }

    {
        let (_, mut processor) = create_task_processor(Arc::clone(&config_state), None);
        let result = process_single_task(
            &mut processor,
            AdminTask::DeleteUpstream(upstream2_name.to_string()),
        )
        .await;

        assert!(result.is_ok(), "删除上游2失败: {:?}", result);
    }

    // 15. 验证所有资源都已删除
    {
        let config = config_state.read().await;
        assert!(!config
            .upstreams
            .iter()
            .any(|u| u.name == upstream1_name || u.name == upstream2_name));
        assert!(!config
            .upstream_groups
            .iter()
            .any(|g| g.name == group1_name || g.name == group2_name));
        assert!(!config
            .http_server
            .forwards
            .iter()
            .any(|f| f.name == forward1_name || f.name == forward2_name));
    }
}
