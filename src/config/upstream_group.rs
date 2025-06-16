use crate::{
    config::{defaults::default_weight, http_client::HttpClientConfig, validation},
    r#const::balance_strategy_labels,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

// 上游组配置
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[validate(schema(function = "validation::validate_weighted_round_robin"))]
#[serde(rename_all = "lowercase")]
pub struct UpstreamGroupConfig {
    // 上游组名称
    #[validate(length(min = 1, message = "Upstream group name cannot be empty"))]
    pub name: String,
    // 上游引用列表
    #[validate(length(min = 1), nested)]
    pub upstreams: Vec<UpstreamRef>,
    // 负载均衡策略
    #[serde(default)]
    pub balance: BalanceConfig,
    // HTTP客户端配置
    #[serde(default)]
    #[validate(nested)]
    pub http_client: HttpClientConfig,
}

// 上游引用
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "lowercase")]
pub struct UpstreamRef {
    // 上游名称
    #[validate(length(min = 1, message = "Upstream name in group cannot be empty"))]
    pub name: String,
    // 权重（用于加权轮询策略）
    #[serde(default = "default_weight")]
    #[validate(range(min = 1, max = 65535, message = "Weight must be between 1 and 65535"))]
    pub weight: u32,
}

// 负载均衡策略配置
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct BalanceConfig {
    // 策略类型
    #[serde(default)]
    pub strategy: BalanceStrategy,
}

// 负载均衡策略类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum BalanceStrategy {
    // 轮询
    #[serde(rename = "roundrobin")]
    RoundRobin,
    // 加权轮询
    #[serde(rename = "weighted_roundrobin")]
    WeightedRoundRobin,
    // 随机
    Random,
    // 响应时间感知
    #[serde(rename = "response_aware")]
    ResponseAware,
    // 故障转移
    #[serde(rename = "failover")]
    Failover,
}

impl Default for BalanceStrategy {
    fn default() -> Self {
        Self::RoundRobin
    }
}

// 将 BalanceStrategy 转换为字符串标签
impl BalanceStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RoundRobin => balance_strategy_labels::ROUND_ROBIN,
            Self::WeightedRoundRobin => balance_strategy_labels::WEIGHTED_ROUND_ROBIN,
            Self::Random => balance_strategy_labels::RANDOM,
            Self::ResponseAware => balance_strategy_labels::RESPONSE_AWARE,
            Self::Failover => balance_strategy_labels::FAILOVER,
        }
    }
}
