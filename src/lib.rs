// 导出所有公共模块
pub mod admin;
pub mod args;
pub mod balancer;
pub mod breaker;
pub mod config;
pub mod r#const;
pub mod error;
pub mod metrics;
pub mod server;
pub mod upstream;

// 重新导出单例指标
pub use crate::metrics::METRICS;
