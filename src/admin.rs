use crate::error::AppError;
use crate::metrics::METRICS;
use async_trait::async_trait;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use prometheus::{Encoder, TextEncoder};
use socket2::{Domain, Protocol, Socket, Type};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio_graceful_shutdown::{IntoSubsystem, SubsystemHandle};
use tracing::{error, info};

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

#[async_trait]
impl IntoSubsystem<AppError> for AdminServer {
    async fn run(self, subsys: SubsystemHandle) -> Result<(), AppError> {
        // 创建路由
        let app = Router::new()
            .route("/health", get(health_handler))
            .route("/metrics", get(metrics_handler));

        // 使用 socket2 创建 TCP 监听器
        // 根据地址类型确定域
        let domain = if self.addr.is_ipv6() {
            Domain::IPV6
        } else {
            Domain::IPV4
        };

        // 创建 socket
        let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))
            .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        // 设置 SO_REUSEADDR 选项 (所有平台)
        socket
            .set_reuse_address(true)
            .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        // 在 Linux 平台上设置 SO_REUSEPORT 选项
        #[cfg(target_os = "linux")]
        socket
            .set_reuse_port(true)
            .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        // 绑定到地址
        let addr = self.addr.into();
        socket
            .bind(&addr)
            .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        // 开始监听
        socket
            .listen(1024)
            .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        // 设置为非阻塞模式
        socket
            .set_nonblocking(true)
            .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        // 将 socket2::Socket 转换为 std::net::TcpListener
        let std_listener: std::net::TcpListener = socket.into();

        // 将 std::net::TcpListener 转换为 tokio::net::TcpListener
        let listener = TcpListener::from_std(std_listener).map_err(AppError::Io)?;

        info!("Admin service listening on {}", self.addr);

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

    // 返回指标
    match String::from_utf8(buffer) {
        Ok(metrics_text) => (StatusCode::OK, metrics_text).into_response(),
        Err(e) => {
            error!("Metrics UTF-8 conversion failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
