use crate::api::v1::{api_routes, openapi_routes};
use crate::config::Config;
use crate::error::AppError;
use crate::metrics::METRICS;
use crate::server::create_tcp_listener;
use async_trait::async_trait;
use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use prometheus::{Encoder, TextEncoder};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_graceful_shutdown::{IntoSubsystem, SubsystemHandle};
use tracing::{error, info};

const HEALTH_PATH: &str = "/health";
const METRICS_PATH: &str = "/metrics";

// 管理服务
pub struct AdminServer {
    // 是否开启调试模式
    debug: bool,
    // 监听地址
    addr: SocketAddr,
    // 配置
    config: Arc<RwLock<Config>>,
}

impl AdminServer {
    // 创建新的管理服务
    pub fn new(debug: bool, addr: SocketAddr, config: Arc<RwLock<Config>>) -> Self {
        Self {
            addr,
            config,
            debug,
        }
    }
}

#[async_trait]
impl IntoSubsystem<AppError> for AdminServer {
    async fn run(self, subsys: SubsystemHandle) -> Result<(), AppError> {
        // 创建路由
        let mut app = Router::new()
            .route(HEALTH_PATH, get(health_handler))
            .route(METRICS_PATH, get(metrics_handler))
            // 添加 API v1 路由
            .merge(api_routes(self.config.clone()));

        // 如果开启调试模式，添加 OpenAPI UI
        if self.debug {
            app = app.merge(openapi_routes());
        }

        // 创建 TCP 监听器
        let listener = create_tcp_listener(self.addr, u16::MAX.into())?;

        info!("Admin service listening on {:?}", self.addr);

        // 使用tokio::select!监听服务器和关闭信号
        tokio::select! {
            result = axum::serve(listener, app) => {
                if let Err(e) = result {
                    error!("Admin service error: {}", e);
                } else {
                    info!("Admin service completed normally");
                }
                Ok(())
            }
            _ = subsys.on_shutdown_requested() => {
                info!("Shutdown requested, stopping admin service");
                Ok(())
            }
        }
    }
}

// 健康检查处理程序
async fn health_handler() -> &'static str {
    "OK"
}

// 指标处理函数
async fn metrics_handler() -> Response {
    // 创建编码器
    let encoder = TextEncoder::new();

    // 收集指标
    let metric_families = METRICS.registry().gather();

    // 预估缓冲区大小，避免多次重新分配
    // 每个指标家族平均大约需要 200 字节
    let estimated_size = metric_families.len() * 200;
    let mut buffer = Vec::with_capacity(estimated_size);

    // 编码指标
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        error!("Failed to encode metrics: {}", e);
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    // Prometheus指标数据总是有效的UTF-8，所以可以直接返回字节
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        buffer,
    )
        .into_response()
}
