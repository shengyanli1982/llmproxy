use crate::config::{Config, ForwardConfig, UpstreamConfig, UpstreamGroupConfig};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

// 配置状态，使用读写锁保护
pub type ConfigState = Arc<RwLock<Config>>;

// API 任务类型
pub enum AdminTask {
    // 上游管理任务
    CreateUpstream(UpstreamConfig),
    UpdateUpstream(String, UpstreamConfig),
    DeleteUpstream(String),

    // 上游组管理任务
    CreateUpstreamGroup(UpstreamGroupConfig),
    UpdateUpstreamGroup(String, UpstreamGroupConfig),
    DeleteUpstreamGroup(String),

    // 转发规则管理任务
    CreateForward(ForwardConfig),
    UpdateForward(String, ForwardConfig),
    DeleteForward(String),
}

// 服务器管理任务类型
pub enum ServerManagerTask {
    // 更新服务器列表
    UpdateServers,
    // 停止特定服务器
    StopServer(String),
    // 启动特定服务器
    StartServer(ForwardConfig),
}

// 服务器管理任务发送器
pub type ServerManagerSender = mpsc::Sender<ServerManagerTask>;

// 任务处理结果
pub type TaskResult = Result<(), super::error::ApiError>;

// 任务处理器
pub struct TaskProcessor {
    // 任务接收器
    pub receiver: mpsc::Receiver<AdminTask>,
    // 配置状态
    pub config: ConfigState,
}

// 任务发送器
pub type TaskSender = mpsc::Sender<AdminTask>;

// 分页查询参数
#[derive(Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

// 分页响应包装
#[derive(Serialize)]
pub struct PaginatedResponse<T> {
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub data: Vec<T>,
}

impl<T> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, query: &PaginationQuery, total: usize) -> Self {
        Self {
            total,
            page: query.page,
            page_size: query.page_size,
            data: items,
        }
    }
}

// 默认分页值
fn default_page() -> usize {
    1
}

fn default_page_size() -> usize {
    10
}

// 路径参数
#[derive(Deserialize)]
pub struct NameParam {
    pub name: String,
}
