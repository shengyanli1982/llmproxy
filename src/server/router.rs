use crate::{config::ForwardConfig, error::AppError};
use radixmap::RadixMap;
use std::collections::HashSet;
use tracing::debug;

// 路由结果
#[derive(Debug, Clone, Copy)]
pub struct RoutingResult<'a> {
    // 目标上游组
    pub target_group: &'a str,
    // 是否使用了默认组
    pub is_default: bool,
}

// 路由器结构
pub struct Router {
    // 路径映射表
    path_map: RadixMap<String>,
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
            path_map,
            default_group,
        })
    }

    // 根据请求路径获取目标上游组
    #[inline(always)]
    pub fn get_target_group<'a>(&'a self, path: &str) -> RoutingResult<'a> {
        // 查找匹配的路由规则
        // 使用 .as_bytes() 将 &str 转换为 &[u8]
        if let Some(target_group) = self.path_map.get(path.as_bytes()) {
            debug!("Routing matched: {:?} -> {:?}", path, target_group);
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
        RoutingResult {
            target_group: &self.default_group,
            is_default: true,
        }
    }
}
