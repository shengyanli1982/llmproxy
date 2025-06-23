use crate::{config::ForwardConfig, error::AppError, upstream::UpstreamManager};
use std::{net::SocketAddr, sync::Arc};
use tokio_graceful_shutdown::{IntoSubsystem, SubsystemHandle};
use tracing::{error, info};

use super::{
    router::Router,
    utils::{apply_middlewares, build_router, create_tcp_listener},
};

// 转发服务状态
pub struct ForwardState {
    // 上游管理器
    pub upstream_manager: Arc<UpstreamManager>,
    // 转发配置
    pub config: ForwardConfig,
    // 路由器
    pub router: Router,
}

// 转发服务
pub struct ForwardServer {
    // 监听地址
    addr: SocketAddr,
    // 服务状态
    state: Arc<ForwardState>,
}

impl ForwardServer {
    // 创建新的转发服务
    pub fn new(
        config: ForwardConfig,
        upstream_manager: Arc<UpstreamManager>,
    ) -> Result<Self, AppError> {
        // 解析监听地址
        let addr = format!("{}:{}", config.address, config.port)
            .parse()
            .map_err(|e| AppError::Config(format!("Invalid listening address: {:?}", e)))?;

        // 创建路由器(转发路由，不是 axum 的路由)
        let router = Router::new(&config)?;

        let state = Arc::new(ForwardState {
            upstream_manager,
            config,
            router,
        });

        Ok(Self { addr, state })
    }

    // 获取服务器监听地址
    #[inline(always)]
    pub fn get_addr(&self) -> &SocketAddr {
        &self.addr
    }

    // 获取服务器状态
    #[inline(always)]
    pub fn get_state(&self) -> &Arc<ForwardState> {
        &self.state
    }
}

#[async_trait::async_trait]
impl IntoSubsystem<AppError> for ForwardServer {
    async fn run(self, subsys: SubsystemHandle) -> Result<(), AppError> {
        // 创建路由
        let app = build_router(self.state.clone());

        // 应用中间件
        let app = apply_middlewares(app, &self.state);

        // 创建 TCP 监听器
        let listener = create_tcp_listener(self.addr, u16::MAX.into())?;

        info!(
            "Forwarding service {:?} listening on {:?}",
            self.state.config.name, self.addr
        );

        // 使用tokio::select!监听服务器和关闭信号
        tokio::select! {
            result = axum::serve(listener, app) => {
                if let Err(e) = result {
                    error!("Forwarding service error: {}", e);
                } else {
                    info!("Forwarding service completed normally");
                }
                Ok(())
            }
            _ = subsys.on_shutdown_requested() => {
                info!("Shutdown requested, stopping forwarding service");
                Ok(())
            }
        }
    }
}
