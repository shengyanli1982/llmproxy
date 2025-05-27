use crate::error::AppError;
use crate::metrics::METRICS;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use prometheus::{Encoder, TextEncoder};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_graceful_shutdown::{IntoSubsystem, SubsystemHandle};
use tracing::{error, info};

// 管理服务状态
#[derive(Clone)]
struct AdminState {
    // 无需任何字段
}

// 管理服务
pub struct AdminServer {
    // 监听地址
    addr: SocketAddr,
}

impl AdminServer {
    // 创建新的管理服务
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }
}

#[async_trait::async_trait]
impl IntoSubsystem<AppError> for AdminServer {
    async fn run(self, subsys: SubsystemHandle) -> Result<(), AppError> {
        // 创建服务状态
        let state = Arc::new(AdminState {});

        // 创建路由
        let app = Router::new()
            .route("/health", get(health_handler))
            .route("/metrics", get(metrics_handler))
            .with_state(state);

        // 绑定TCP监听器
        let listener = match TcpListener::bind(self.addr).await {
            Ok(listener) => {
                info!("Admin service listening on {}", self.addr);
                listener
            }
            Err(e) => {
                error!("Failed to bind admin service: {}", e);
                return Err(AppError::Io(e));
            }
        };

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

    // 编码指标
    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        error!("Failed to encode metrics: {}", e);
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    // 返回指标
    match String::from_utf8(buffer) {
        Ok(metrics_text) => (StatusCode::OK, metrics_text).into_response(),
        Err(e) => {
            error!("Metrics UTF-8 conversion failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
