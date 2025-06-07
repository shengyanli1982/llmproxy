use llmproxy::{
    apis::v1::{
        error::ApiError,
        types::{AdminTask, ConfigState, ServerManagerSender, ServerManagerTask, TaskProcessor},
        validation::ConfigValidation,
    },
    config::{
        AuthConfig, AuthType, BalanceConfig, BalanceStrategy, BreakerConfig, Config, ForwardConfig,
        HeaderOpType, HeaderOperation, HttpClientConfig, RateLimitConfig, TimeoutConfig,
        UpstreamConfig, UpstreamGroupConfig, UpstreamRef,
    },
};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

/// 创建测试配置和必要的组件
pub fn create_test_config() -> Config {
    let upstream_config = UpstreamConfig {
        name: "test_upstream".to_string(),
        url: "http://localhost:8080".to_string(),
        id: Uuid::new_v4().to_string(),
        auth: Some(AuthConfig {
            r#type: AuthType::Bearer,
            token: Some("test_token".to_string()),
            username: None,
            password: None,
        }),
        headers: vec![HeaderOperation {
            op: HeaderOpType::Insert,
            key: "X-Test-Header".to_string(),
            value: Some("test-value".to_string()),
        }],
        breaker: Some(BreakerConfig {
            threshold: 0.5,
            cooldown: 30,
        }),
    };

    let upstream_ref = UpstreamRef {
        name: "test_upstream".to_string(),
        weight: 1,
    };

    let group_config = UpstreamGroupConfig {
        name: "test_group".to_string(),
        upstreams: vec![upstream_ref],
        balance: BalanceConfig {
            strategy: BalanceStrategy::RoundRobin,
        },
        http_client: HttpClientConfig::default(),
    };

    let forward_config = ForwardConfig {
        name: "test_forward".to_string(),
        port: 3000,
        address: "127.0.0.1".to_string(),
        upstream_group: "test_group".to_string(),
        ratelimit: RateLimitConfig {
            enabled: false,
            per_second: 100,
            burst: 200,
        },
        timeout: TimeoutConfig { connect: 5 },
    };

    Config {
        http_server: llmproxy::config::HttpServerConfig {
            forwards: vec![forward_config],
            admin: llmproxy::config::AdminConfig {
                port: 9000,
                address: "127.0.0.1".to_string(),
                timeout: TimeoutConfig { connect: 5 },
            },
        },
        upstreams: vec![upstream_config],
        upstream_groups: vec![group_config],
    }
}

/// 创建测试环境
pub fn setup_test_env() -> (
    ConfigState,
    mpsc::Sender<AdminTask>,
    mpsc::Receiver<ServerManagerTask>,
) {
    let config = create_test_config();
    let config_state = Arc::new(RwLock::new(config));

    // 创建任务通道
    let (task_sender, _task_receiver) = mpsc::channel(500);

    // 创建服务器管理器通道
    let (_server_sender, server_receiver) = mpsc::channel(500);

    (config_state, task_sender, server_receiver)
}

/// 创建任务处理器
pub fn create_task_processor(
    config_state: ConfigState,
    server_manager_sender: Option<ServerManagerSender>,
) -> (mpsc::Sender<AdminTask>, TaskProcessor) {
    let (sender, receiver) = mpsc::channel(500);

    let processor = TaskProcessor {
        receiver,
        config: Arc::clone(&config_state),
        sender: server_manager_sender,
    };

    (sender, processor)
}

/// 模拟任务处理（通过修改配置直接实现，而不是调用私有方法）
pub async fn process_single_task(
    processor: &mut TaskProcessor,
    task: AdminTask,
) -> Result<(), ApiError> {
    match task {
        AdminTask::CreateUpstream(upstream) => {
            // 检查名称是否已存在
            let mut config_guard = processor.config.write().await;

            if config_guard
                .upstreams
                .iter()
                .any(|u| u.name == upstream.name)
            {
                return Err(ApiError::AlreadyExists(format!(
                    "Upstream with name '{}' already exists",
                    upstream.name
                )));
            }

            // 校验上游配置
            if let Err(e) = config_guard.validate_upstream_config(&upstream) {
                return Err(e);
            }

            // 添加到配置中
            config_guard.upstreams.push(upstream);
            Ok(())
        }
        AdminTask::UpdateUpstream(name, mut upstream) => {
            let mut config_guard = processor.config.write().await;

            // 查找要更新的上游索引
            let index = config_guard
                .upstreams
                .iter()
                .position(|u| u.name == name)
                .ok_or_else(|| ApiError::NotFound(format!("Upstream '{}' not found", name)))?;

            // 确保名称一致
            if upstream.name != name {
                return Err(ApiError::ValidationError(format!(
                    "Name in URL ('{}') and request body ('{}') must match",
                    name, upstream.name
                )));
            }

            // 保留原始ID
            upstream.id = config_guard.upstreams[index].id.clone();

            // 校验更新后的配置
            if let Err(e) = config_guard.validate_upstream_config(&upstream) {
                return Err(e);
            }

            // 更新配置
            config_guard.upstreams[index] = upstream;
            Ok(())
        }
        AdminTask::DeleteUpstream(name) => {
            let mut config_guard = processor.config.write().await;

            // 检查是否存在
            if !config_guard.upstreams.iter().any(|u| u.name == name) {
                // 幂等性处理：如果已经不存在，视为成功
                return Ok(());
            }

            // 检查是否有上游组引用此上游
            let mut referenced_by = Vec::new();

            for group in &config_guard.upstream_groups {
                if group.upstreams.iter().any(|u| u.name == name) {
                    referenced_by.push(format!("upstream-group:{}", group.name));
                }
            }

            if !referenced_by.is_empty() {
                return Err(ApiError::StillReferenced {
                    resource_type: "Upstream".into(),
                    name,
                    referenced_by,
                });
            }

            // 执行删除
            config_guard.upstreams.retain(|u| u.name != name);
            Ok(())
        }
        AdminTask::CreateUpstreamGroup(group) => {
            let mut config_guard = processor.config.write().await;

            // 检查名称是否已存在
            if config_guard
                .upstream_groups
                .iter()
                .any(|g| g.name == group.name)
            {
                return Err(ApiError::AlreadyExists(format!(
                    "Upstream group with name '{}' already exists",
                    group.name
                )));
            }

            // 检查引用的上游是否存在
            for upstream_ref in &group.upstreams {
                if !config_guard
                    .upstreams
                    .iter()
                    .any(|u| u.name == upstream_ref.name)
                {
                    return Err(ApiError::ReferenceNotFound {
                        resource_type: "Upstream".into(),
                        name: upstream_ref.name.clone(),
                    });
                }
            }

            // 校验上游组配置
            if let Err(e) = config_guard.validate_upstream_group_config(&group) {
                return Err(e);
            }

            // 添加到配置中
            config_guard.upstream_groups.push(group);
            Ok(())
        }
        AdminTask::UpdateUpstreamGroup(name, group) => {
            let mut config_guard = processor.config.write().await;

            // 查找要更新的上游组索引
            let index = config_guard
                .upstream_groups
                .iter()
                .position(|g| g.name == name)
                .ok_or_else(|| {
                    ApiError::NotFound(format!("Upstream group '{}' not found", name))
                })?;

            // 确保名称一致
            if group.name != name {
                return Err(ApiError::ValidationError(format!(
                    "Name in URL ('{}') and request body ('{}') must match",
                    name, group.name
                )));
            }

            // 检查引用的上游是否存在
            for upstream_ref in &group.upstreams {
                if !config_guard
                    .upstreams
                    .iter()
                    .any(|u| u.name == upstream_ref.name)
                {
                    return Err(ApiError::ReferenceNotFound {
                        resource_type: "Upstream".into(),
                        name: upstream_ref.name.clone(),
                    });
                }
            }

            // 校验更新后的配置
            if let Err(e) = config_guard.validate_upstream_group_config(&group) {
                return Err(e);
            }

            // 更新配置
            config_guard.upstream_groups[index] = group;
            Ok(())
        }
        AdminTask::DeleteUpstreamGroup(name) => {
            let mut config_guard = processor.config.write().await;

            // 检查是否存在
            if !config_guard.upstream_groups.iter().any(|g| g.name == name) {
                // 幂等性处理：如果已经不存在，视为成功
                return Ok(());
            }

            // 检查是否有转发规则引用此上游组
            let mut referenced_by = Vec::new();

            for forward in &config_guard.http_server.forwards {
                if forward.upstream_group == name {
                    referenced_by.push(format!("forward:{}", forward.name));
                }
            }

            if !referenced_by.is_empty() {
                return Err(ApiError::StillReferenced {
                    resource_type: "UpstreamGroup".into(),
                    name,
                    referenced_by,
                });
            }

            // 执行删除
            config_guard.upstream_groups.retain(|g| g.name != name);
            Ok(())
        }
        AdminTask::CreateForward(forward) => {
            let mut config_guard = processor.config.write().await;

            // 检查名称是否已存在
            if config_guard
                .http_server
                .forwards
                .iter()
                .any(|f| f.name == forward.name)
            {
                return Err(ApiError::AlreadyExists(format!(
                    "Forward with name '{}' already exists",
                    forward.name
                )));
            }

            // 检查引用的上游组是否存在
            if !config_guard
                .upstream_groups
                .iter()
                .any(|g| g.name == forward.upstream_group)
            {
                return Err(ApiError::ReferenceNotFound {
                    resource_type: "UpstreamGroup".into(),
                    name: forward.upstream_group.clone(),
                });
            }

            // 校验转发规则配置
            if let Err(e) = config_guard.validate_forward_config(&forward) {
                return Err(e);
            }

            // 添加到配置中
            config_guard.http_server.forwards.push(forward.clone());

            // 发送服务器启动任务
            if let Some(sender) = &processor.sender {
                let _ = sender.send(ServerManagerTask::StartServer(forward)).await;
            }

            Ok(())
        }
        AdminTask::UpdateForward(name, forward) => {
            let mut config_guard = processor.config.write().await;

            // 查找要更新的转发规则索引
            let index = config_guard
                .http_server
                .forwards
                .iter()
                .position(|f| f.name == name)
                .ok_or_else(|| ApiError::NotFound(format!("Forward '{}' not found", name)))?;

            // 确保名称一致
            if forward.name != name {
                return Err(ApiError::ValidationError(format!(
                    "Name in URL ('{}') and request body ('{}') must match",
                    name, forward.name
                )));
            }

            // 检查引用的上游组是否存在
            if !config_guard
                .upstream_groups
                .iter()
                .any(|g| g.name == forward.upstream_group)
            {
                return Err(ApiError::ReferenceNotFound {
                    resource_type: "UpstreamGroup".into(),
                    name: forward.upstream_group.clone(),
                });
            }

            // 校验更新后的配置
            if let Err(e) = config_guard.validate_forward_config(&forward) {
                return Err(e);
            }

            // 检查是否需要重新启动服务器
            let need_restart = config_guard.http_server.forwards[index].port != forward.port
                || config_guard.http_server.forwards[index].address != forward.address;

            // 更新配置
            config_guard.http_server.forwards[index] = forward.clone();

            // 如果需要重新启动服务器，发送停止和启动任务
            if need_restart && processor.sender.is_some() {
                let sender = processor.sender.as_ref().unwrap();

                // 发送停止任务
                let _ = sender
                    .send(ServerManagerTask::StopServer(name.clone()))
                    .await;

                // 发送启动任务
                let _ = sender.send(ServerManagerTask::StartServer(forward)).await;
            }

            Ok(())
        }
        AdminTask::DeleteForward(name) => {
            let mut config_guard = processor.config.write().await;

            // 检查是否存在
            if !config_guard
                .http_server
                .forwards
                .iter()
                .any(|f| f.name == name)
            {
                // 幂等性处理：如果已经不存在，视为成功
                return Ok(());
            }

            // 执行删除
            config_guard.http_server.forwards.retain(|f| f.name != name);

            // 发送服务器停止任务
            if let Some(sender) = &processor.sender {
                let _ = sender.send(ServerManagerTask::StopServer(name)).await;
            }

            Ok(())
        }
    }
}

/// 创建服务器管理器发送器
pub fn create_server_manager_sender() -> (ServerManagerSender, mpsc::Receiver<ServerManagerTask>) {
    mpsc::channel(500)
}
