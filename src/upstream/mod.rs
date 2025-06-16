mod builder;
mod http_client;
mod manager;

// 重新导出公共API，保持原有调用方式不变
pub use manager::UpstreamManager;

// 如果有其他在crate外部使用的类型或函数，也需要在这里重新导出
