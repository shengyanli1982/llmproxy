use crate::{config::ForwardConfig, error::AppError};
use radixmap::RadixMap;
use std::collections::HashSet;
use tokio::sync::RwLock;
use tracing::debug;

// 路由结果
#[derive(Debug, Clone)]
pub struct RoutingResult {
    // 目标上游组
    pub target_group: String,
    // 是否使用了默认组
    pub is_default: bool,
}

// 路由器结构
pub struct Router {
    // 路径映射表
    path_map: RwLock<RadixMap<String>>,
    // 默认上游组
    default_group: String,
}

impl Router {
    // 创建新的路由器
    pub fn new(config: &ForwardConfig) -> Result<Self, AppError> {
        let mut path_map = RadixMap::new();
        let default_group = config.default_group.clone();

        // 处理路由规则
        if let Some(routing_rules) = &config.routing {
            let mut paths = HashSet::new();
            // 明确使用 .iter() 来帮助编译器推断生命周期
            for rule in routing_rules.iter() {
                // 检查路径唯一性
                if !paths.insert(&rule.path) {
                    return Err(AppError::Config(format!(
                        "Duplicate routing path found: {:?}",
                        rule.path
                    )));
                }

                if let Err(e) = path_map.insert(rule.path.clone(), rule.target_group.clone()) {
                    return Err(AppError::Config(format!(
                        "Error adding route: {:?} -> {:?}, error: {}",
                        rule.path, rule.target_group, e
                    )));
                }

                debug!(
                    "Added routing rule: {:?} -> {:?}",
                    rule.path, rule.target_group
                );
            }
        }

        Ok(Self {
            path_map: RwLock::new(path_map),
            default_group,
        })
    }
    // 创建和更新路由规则
    pub async fn insert_or_update_route(
        &self,
        path: String,
        target_group: String,
    ) -> Result<(), AppError> {
        // 获取写锁
        let mut path_map = self.path_map.write().await;
        let _ = path_map.insert(path, target_group);
        // 锁会在这里自动释放
        Ok(())
    }

    // 删除路由规则
    pub async fn remove_route(&self, path: &str) -> Result<(), AppError> {
        // 获取写锁
        let mut path_map = self.path_map.write().await;
        path_map.remove(path.as_bytes());
        // 锁会在这里自动释放
        Ok(())
    }

    // 根据请求路径获取目标上游组
    #[inline(always)]
    pub async fn get_target_group(&self, path: &str) -> RoutingResult {
        // 获取读锁
        let path_map_read = self.path_map.read().await;

        // 查找匹配的路由规则
        // 使用 .as_bytes() 将 &str 转换为 &[u8]
        if let Some(target_group) = path_map_read.get(path.as_bytes()) {
            debug!("Routing matched: {:?} -> {:?}", path, target_group);
            // 克隆目标上游组的值，避免生命周期问题
            let target_group = target_group.to_owned();
            drop(path_map_read);

            return RoutingResult {
                target_group,
                is_default: false,
            };
        }

        // 没有匹配规则，使用默认上游组
        debug!(
            "No routing rule matched for path: {:?}, using default group: {:?}",
            path, self.default_group
        );

        drop(path_map_read);

        RoutingResult {
            target_group: self.default_group.clone(),
            is_default: true,
        }
    }
}
