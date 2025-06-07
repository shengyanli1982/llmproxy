use crate::apis::v1::{
    error::ApiError,
    types::{AdminTask, TaskProcessor, TaskResult},
    validation::ConfigValidation,
};
use crate::config::{Config, ForwardConfig, UpstreamConfig, UpstreamGroupConfig};
use std::collections::HashSet;
use tracing::{debug, error, info};

/// 任务服务
pub struct TaskService;

impl TaskService {
    pub async fn run(processor: &mut TaskProcessor) {
        info!("Starting admin API task processor");

        while let Some(task) = processor.receiver.recv().await {
            let result = match task {
                AdminTask::CreateUpstream(upstream) => {
                    Self::create_upstream(processor, upstream).await
                }
                AdminTask::UpdateUpstream(name, upstream) => {
                    Self::update_upstream(processor, name, upstream).await
                }
                AdminTask::DeleteUpstream(name) => Self::delete_upstream(processor, name).await,

                AdminTask::CreateUpstreamGroup(group) => {
                    Self::create_upstream_group(processor, group).await
                }
                AdminTask::UpdateUpstreamGroup(name, group) => {
                    Self::update_upstream_group(processor, name, group).await
                }
                AdminTask::DeleteUpstreamGroup(name) => {
                    Self::delete_upstream_group(processor, name).await
                }

                AdminTask::CreateForward(forward) => Self::create_forward(processor, forward).await,
                AdminTask::UpdateForward(name, forward) => {
                    Self::update_forward(processor, name, forward).await
                }
                AdminTask::DeleteForward(name) => Self::delete_forward(processor, name).await,
            };

            if let Err(e) = result {
                error!("Task processing error: {}", e);
            }
        }

        info!("Admin API task processor stopped");
    }

    pub fn get_config(processor: &TaskProcessor) -> Result<Config, ApiError> {
        processor
            .config
            .read()
            .map(|config| config.clone())
            .map_err(|e| ApiError::InternalError(format!("Failed to read config: {}", e)))
    }

    // ===== 上游处理方法 =====
    async fn create_upstream(
        processor: &mut TaskProcessor,
        upstream: UpstreamConfig,
    ) -> TaskResult {
        debug!("Creating upstream: {}", upstream.name);

        let mut config = processor
            .config
            .write()
            .map_err(|e| ApiError::InternalError(format!("Failed to acquire write lock: {}", e)))?;

        // 检查名称是否已存在
        if config.upstreams.iter().any(|u| u.name == upstream.name) {
            return Err(ApiError::AlreadyExists(format!(
                "Upstream with name '{}' already exists",
                upstream.name
            )));
        }

        // 校验上游配置
        if let Err(e) = config.validate_upstream_config(&upstream) {
            return Err(ApiError::ValidationError(e.to_string()));
        }

        // 添加到配置中
        config.upstreams.push(upstream);

        Ok(())
    }

    // 更新上游
    async fn update_upstream(
        processor: &mut TaskProcessor,
        name: String,
        mut upstream: UpstreamConfig,
    ) -> TaskResult {
        debug!("Updating upstream: {}", name);

        let mut config = processor
            .config
            .write()
            .map_err(|e| ApiError::InternalError(format!("Failed to acquire write lock: {}", e)))?;

        // 查找要更新的上游索引
        let index = config
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
        upstream.id = config.upstreams[index].id.clone();

        // 校验更新后的配置
        if let Err(e) = config.validate_upstream_config(&upstream) {
            return Err(ApiError::ValidationError(e.to_string()));
        }

        // 更新配置
        config.upstreams[index] = upstream;

        Ok(())
    }

    // 删除上游
    async fn delete_upstream(processor: &mut TaskProcessor, name: String) -> TaskResult {
        debug!("Deleting upstream: {}", name);

        let mut config = processor
            .config
            .write()
            .map_err(|e| ApiError::InternalError(format!("Failed to acquire write lock: {}", e)))?;

        // 检查是否存在
        if !config.upstreams.iter().any(|u| u.name == name) {
            // 幂等性处理：如果已经不存在，视为成功
            return Ok(());
        }

        // 检查是否有上游组引用此上游
        let mut referenced_by = Vec::new();

        for group in &config.upstream_groups {
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
        config.upstreams.retain(|u| u.name != name);

        Ok(())
    }

    // ===== 上游组处理方法 =====

    // 创建上游组
    async fn create_upstream_group(
        processor: &mut TaskProcessor,
        group: UpstreamGroupConfig,
    ) -> TaskResult {
        debug!("Creating upstream group: {}", group.name);

        let mut config = processor
            .config
            .write()
            .map_err(|e| ApiError::InternalError(format!("Failed to acquire write lock: {}", e)))?;

        // 检查名称是否已存在
        if config.upstream_groups.iter().any(|g| g.name == group.name) {
            return Err(ApiError::AlreadyExists(format!(
                "Upstream group with name '{}' already exists",
                group.name
            )));
        }

        // 检查引用的上游是否存在
        let upstream_names: HashSet<_> = config.upstreams.iter().map(|u| u.name.clone()).collect();

        for upstream_ref in &group.upstreams {
            if !upstream_names.contains(&upstream_ref.name) {
                return Err(ApiError::ReferenceNotFound {
                    resource_type: "Upstream".into(),
                    name: upstream_ref.name.clone(),
                });
            }
        }

        // 校验上游组配置
        if let Err(e) = config.validate_upstream_group_config(&group) {
            return Err(ApiError::ValidationError(e.to_string()));
        }

        // 添加到配置中
        config.upstream_groups.push(group);

        Ok(())
    }

    // 更新上游组
    async fn update_upstream_group(
        processor: &mut TaskProcessor,
        name: String,
        group: UpstreamGroupConfig,
    ) -> TaskResult {
        debug!("Updating upstream group: {}", name);

        let mut config = processor
            .config
            .write()
            .map_err(|e| ApiError::InternalError(format!("Failed to acquire write lock: {}", e)))?;

        // 查找要更新的上游组索引
        let index = config
            .upstream_groups
            .iter()
            .position(|g| g.name == name)
            .ok_or_else(|| ApiError::NotFound(format!("Upstream group '{}' not found", name)))?;

        // 确保名称一致
        if group.name != name {
            return Err(ApiError::ValidationError(format!(
                "Name in URL ('{}') and request body ('{}') must match",
                name, group.name
            )));
        }

        // 检查引用的上游是否存在
        let upstream_names: HashSet<_> = config.upstreams.iter().map(|u| u.name.clone()).collect();

        for upstream_ref in &group.upstreams {
            if !upstream_names.contains(&upstream_ref.name) {
                return Err(ApiError::ReferenceNotFound {
                    resource_type: "Upstream".into(),
                    name: upstream_ref.name.clone(),
                });
            }
        }

        // 校验上游组配置
        if let Err(e) = config.validate_upstream_group_config(&group) {
            return Err(ApiError::ValidationError(e.to_string()));
        }

        // 更新配置
        config.upstream_groups[index] = group;

        Ok(())
    }

    // 删除上游组
    async fn delete_upstream_group(processor: &mut TaskProcessor, name: String) -> TaskResult {
        debug!("Deleting upstream group: {}", name);

        let mut config = processor
            .config
            .write()
            .map_err(|e| ApiError::InternalError(format!("Failed to acquire write lock: {}", e)))?;

        // 检查是否存在
        if !config.upstream_groups.iter().any(|g| g.name == name) {
            // 幂等性处理：如果已经不存在，视为成功
            return Ok(());
        }

        // 检查是否有转发规则引用此上游组
        let mut referenced_by = Vec::new();

        for forward in &config.http_server.forwards {
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
        config.upstream_groups.retain(|g| g.name != name);

        Ok(())
    }

    // ===== 转发规则处理方法 =====

    // 创建转发规则
    async fn create_forward(processor: &mut TaskProcessor, forward: ForwardConfig) -> TaskResult {
        debug!("Creating forward: {}", forward.name);

        let mut config = processor
            .config
            .write()
            .map_err(|e| ApiError::InternalError(format!("Failed to acquire write lock: {}", e)))?;

        // 检查名称是否已存在
        if config
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
        let group_names: HashSet<_> = config
            .upstream_groups
            .iter()
            .map(|g| g.name.clone())
            .collect();

        if !group_names.contains(&forward.upstream_group) {
            return Err(ApiError::ReferenceNotFound {
                resource_type: "UpstreamGroup".into(),
                name: forward.upstream_group.clone(),
            });
        }

        // 校验转发规则配置
        if let Err(e) = config.validate_forward_config(&forward) {
            return Err(ApiError::ValidationError(e.to_string()));
        }

        // 添加到配置中
        config.http_server.forwards.push(forward);

        Ok(())
    }

    // 更新转发规则
    async fn update_forward(
        processor: &mut TaskProcessor,
        name: String,
        forward: ForwardConfig,
    ) -> TaskResult {
        debug!("Updating forward: {}", name);

        let mut config = processor
            .config
            .write()
            .map_err(|e| ApiError::InternalError(format!("Failed to acquire write lock: {}", e)))?;

        // 查找要更新的转发规则索引
        let index = config
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
        let group_names: HashSet<_> = config
            .upstream_groups
            .iter()
            .map(|g| g.name.clone())
            .collect();

        if !group_names.contains(&forward.upstream_group) {
            return Err(ApiError::ReferenceNotFound {
                resource_type: "UpstreamGroup".into(),
                name: forward.upstream_group.clone(),
            });
        }

        // 校验更新后的配置
        if let Err(e) = config.validate_forward_config(&forward) {
            return Err(ApiError::ValidationError(e.to_string()));
        }

        // 更新配置
        config.http_server.forwards[index] = forward;

        Ok(())
    }

    // 删除转发规则
    async fn delete_forward(processor: &mut TaskProcessor, name: String) -> TaskResult {
        debug!("Deleting forward: {}", name);

        let mut config = processor
            .config
            .write()
            .map_err(|e| ApiError::InternalError(format!("Failed to acquire write lock: {}", e)))?;

        // 检查是否存在
        if !config.http_server.forwards.iter().any(|f| f.name == name) {
            // 幂等性处理：如果已经不存在，视为成功
            return Ok(());
        }

        // 执行删除
        config.http_server.forwards.retain(|f| f.name != name);

        Ok(())
    }
}
