use crate::{
    config::{http_server::RoutingRule, ForwardConfig},
    error::AppError,
};
use std::collections::{HashMap, HashSet};
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
    // 路径映射表 - 简单实现，使用HashMap代替RadixMap
    path_map: HashMap<String, String>,
    // 默认上游组
    default_group: String,
}

impl Router {
    // 创建新的路由器
    pub fn new(config: &ForwardConfig) -> Result<Self, AppError> {
        let mut path_map = HashMap::new();
        let default_group = config.default_group.clone();

        // 处理路由规则
        if let Some(routing_rules) = &config.routing {
            let mut paths = HashSet::new();

            // 检查路径唯一性并添加到路由表
            for rule in routing_rules {
                let path = rule.path.clone();

                // 检查路径唯一性
                if !paths.insert(path.clone()) {
                    return Err(AppError::Config(format!(
                        "Duplicate routing path found: {}",
                        path
                    )));
                }

                // 添加到路径映射表
                let target = rule.target_group.clone();
                path_map.insert(path.clone(), target.clone());

                debug!("Added routing rule: {} -> {}", path, target);
            }
        }

        Ok(Self {
            path_map,
            default_group,
        })
    }

    // 根据请求路径获取目标上游组
    #[inline(always)]
    pub fn get_target_group(&self, path: &str) -> RoutingResult {
        // 查找匹配的路由规则
        if let Some(target_group) = self.path_map.get(path) {
            debug!("Routing matched: {} -> {}", path, target_group);
            return RoutingResult {
                target_group: target_group.clone(),
                is_default: false,
            };
        }

        // 没有匹配规则，使用默认上游组
        debug!(
            "No routing rule matched for path: {}, using default group: {}",
            path, self.default_group
        );
        RoutingResult {
            target_group: self.default_group.clone(),
            is_default: true,
        }
    }
}
