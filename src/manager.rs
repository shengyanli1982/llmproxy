use crate::apis::v1::types::ServerManagerTask;
use crate::config::{Config, ForwardConfig};
use crate::error::AppError;
use crate::server::ForwardServer;
use crate::upstream::UpstreamManager;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;
use tokio_graceful_shutdown::{IntoSubsystem, SubsystemHandle};
use tracing::{debug, error, info};

pub const SERVER_MANAGER_TASK_CHANNEL_SIZE: usize = 1024;

// 服务器管理器
pub struct ServerManager {
    // 配置引用
    config: Arc<RwLock<Config>>,
    // 上游管理器
    upstream_manager: Arc<UpstreamManager>,
    // 任务接收器
    receiver: mpsc::Receiver<ServerManagerTask>,
    // 活跃的服务器
    active_servers: HashMap<String, (SocketAddr, JoinHandle<()>)>,
    // 退出通知发送器
    shutdown_senders: HashMap<String, tokio::sync::oneshot::Sender<()>>,
}

impl ServerManager {
    // 创建新的服务器管理器
    pub fn new(
        config: Arc<RwLock<Config>>,
        upstream_manager: Arc<UpstreamManager>,
        receiver: mpsc::Receiver<ServerManagerTask>,
    ) -> Self {
        Self {
            config,
            upstream_manager,
            receiver,
            active_servers: HashMap::new(),
            shutdown_senders: HashMap::new(),
        }
    }

    // 创建消息通道
    pub fn create_channel() -> (
        mpsc::Sender<ServerManagerTask>,
        mpsc::Receiver<ServerManagerTask>,
    ) {
        mpsc::channel(SERVER_MANAGER_TASK_CHANNEL_SIZE)
    }

    // 启动初始服务器
    async fn start_initial_servers(&mut self) -> Result<(), AppError> {
        let forwards = {
            let config = self.config.read().await;
            config.http_server.forwards.clone()
        };

        for forward_config in forwards {
            self.start_server(forward_config).await?;
        }
        Ok(())
    }

    // 启动一个新的服务器
    async fn start_server(&mut self, config: ForwardConfig) -> Result<(), AppError> {
        let name = config.name.clone();
        let addr_str = format!("{}:{}", config.address, config.port);

        // 检查是否已经有同名服务器
        if self.active_servers.contains_key(&name) {
            info!("Server '{}' already exists, skipping start", name);
            return Ok(());
        }

        // 检查是否已经有同地址的服务器
        for (server_name, (server_addr, _)) in &self.active_servers {
            if addr_str == server_addr.to_string() {
                error!(
                    "Failed to start server '{}': address {} is already in use by server '{}'",
                    name, addr_str, server_name
                );
                return Err(AppError::Config(format!(
                    "Address {} is already in use by server '{}'",
                    addr_str, server_name
                )));
            }
        }

        // 创建服务器
        let server = match ForwardServer::new(config.clone(), self.upstream_manager.clone()) {
            Ok(server) => server,
            Err(e) => {
                error!("Failed to create server '{}': {}", name, e);
                return Err(e);
            }
        };

        let addr = *server.get_addr();

        // 创建退出通知通道
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        // 存储服务器信息和名称的副本
        let name_clone = name.clone();

        // 启动服务器
        let handle = tokio::spawn(async move {
            let res = server.run_with_shutdown(shutdown_rx).await;
            if let Err(e) = res {
                error!("Server '{}' running error: {}", name_clone, e);
            }
        });

        // 存储服务器信息
        self.active_servers.insert(name.clone(), (addr, handle));
        self.shutdown_senders.insert(name.clone(), shutdown_tx);

        info!("Server '{}' started successfully at {}", name, addr);
        Ok(())
    }

    // 停止一个服务器
    async fn stop_server(&mut self, name: String) -> Result<(), AppError> {
        if let Some(shutdown_tx) = self.shutdown_senders.remove(&name) {
            debug!("Sending stop signal to server '{}'", name);
            // 发送停止信号
            let _ = shutdown_tx.send(());

            // 移除服务器
            if let Some((addr, handle)) = self.active_servers.remove(&name) {
                debug!("Waiting for server '{}' to stop", name);
                // 等待服务器任务完成
                if let Err(e) = handle.await {
                    error!("Error waiting for server '{}' to stop: {}", name, e);
                }
                info!("Server '{}' (address: {}) has stopped", name, addr);
            }
        } else {
            debug!("Server '{}' does not exist or has already stopped", name);
        }

        Ok(())
    }

    // 更新服务器列表
    async fn update_servers(&mut self) -> Result<(), AppError> {
        // 获取需要的配置数据，并在读取后立即释放锁
        let (configured_servers, to_start) = {
            let config = self.config.read().await;

            // 获取当前配置中的所有转发规则名称
            let configured_servers: HashSet<String> = config
                .http_server
                .forwards
                .iter()
                .map(|f| f.name.clone())
                .collect();

            // 找出需要启动的服务器
            let to_start: Vec<ForwardConfig> = config
                .http_server
                .forwards
                .iter()
                .filter(|f| !self.active_servers.contains_key(&f.name))
                .cloned()
                .collect();

            (configured_servers, to_start)
        };

        // 获取当前活跃的服务器名称
        let active_servers: HashSet<String> = self.active_servers.keys().cloned().collect();

        // 找出需要停止的服务器
        let to_stop: Vec<String> = active_servers
            .difference(&configured_servers)
            .cloned()
            .collect();

        // 停止不再需要的服务器
        for name in to_stop {
            debug!("Stopping unnecessary server '{}'", name);
            self.stop_server(name).await?;
        }

        // 启动新配置的服务器
        for config in to_start {
            debug!("Starting newly configured server '{}'", config.name);
            self.start_server(config).await?;
        }

        info!("Server list updated");
        Ok(())
    }
}

#[async_trait]
impl IntoSubsystem<AppError> for ServerManager {
    async fn run(mut self, subsys: SubsystemHandle) -> Result<(), AppError> {
        info!("Starting server manager");

        // 启动初始服务器
        self.start_initial_servers().await?;

        // 处理任务
        loop {
            tokio::select! {
                Some(task) = self.receiver.recv() => {
                    match task {
                        ServerManagerTask::UpdateServers => {
                            if let Err(e) = self.update_servers().await {
                                error!("Failed to update server list: {}", e);
                            }
                        },
                        ServerManagerTask::StopServer(name) => {
                            if let Err(e) = self.stop_server(name).await {
                                error!("Failed to stop server: {}", e);
                            }
                        },
                        ServerManagerTask::StartServer(config) => {
                            if let Err(e) = self.start_server(config).await {
                                error!("Failed to start server: {}", e);
                            }
                        },
                    }
                },
                _ = subsys.on_shutdown_requested() => {
                    info!("Shutdown request received, stopping all servers");

                    // 停止所有服务器
                    for name in self.active_servers.keys().cloned().collect::<Vec<String>>() {
                        if let Err(e) = self.stop_server(name).await {
                            error!("Failed to stop server during shutdown: {}", e);
                        }
                    }

                    break;
                }
            }
        }

        info!("Server manager stopped");
        Ok(())
    }
}
