mod admin;
mod args;
mod balancer;
mod config;
mod r#const;
mod error;
mod metrics;
mod server;
mod upstream;

use crate::admin::AdminServer;
use crate::args::Args;
use crate::config::Config;
use crate::error::AppError;
use crate::server::ForwardServer;
use crate::upstream::UpstreamManager;
use mimalloc::MiMalloc;
use std::process;
use std::sync::Arc;
use tokio_graceful_shutdown::{IntoSubsystem, SubsystemBuilder, Toplevel};
use tracing::{error, info};

// 使用 mimalloc 分配器提高内存效率
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn init_logging(args: &Args) {
    let builder = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_line_number(true);

    // 如果启用调试模式，输出调试信息
    if args.debug {
        builder.with_max_level(tracing::Level::DEBUG)
    } else {
        builder.with_max_level(tracing::Level::INFO)
    }
    .init();
}

// 程序入口
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 解析命令行参数
    let args = Args::parse_args();

    // 初始化日志
    init_logging(&args);

    // 验证参数
    if let Err(e) = args.validation() {
        error!("Invalid command line arguments: {}", e);
        process::exit(1);
    }

    info!("Starting LLMProxy - Large Model Proxy Service");

    // 加载配置
    let config = match Config::from_file(&args.config) {
        Ok(config) => {
            info!("Successfully loaded configuration: {:?}", args.config);
            config
        }
        Err(e) => {
            error!("Failed to load configuration file: {}", e);
            process::exit(1);
        }
    };

    // 如果是测试模式，成功验证配置后退出
    if args.test_config {
        info!("Configuration file validated successfully");
        return Ok(());
    }

    // 创建应用组件
    let components = match create_components(config).await {
        Ok(components) => components,
        Err(e) => {
            error!("Failed to create application components: {}", e);
            process::exit(1);
        }
    };

    // 创建优雅关闭顶层管理器
    let toplevel = Toplevel::new(|s| async move {
        // 启动管理服务子系统
        let admin_server = components.admin_server;
        s.start(SubsystemBuilder::new("admin_server", move |s| async move {
            admin_server.run(s).await
        }));

        // 启动所有转发服务子系统
        for (i, forward_server) in components.forward_servers.into_iter().enumerate() {
            let subsystem_name = format!("forward_server_{}", i);
            s.start(SubsystemBuilder::new(subsystem_name, move |s| async move {
                forward_server.run(s).await
            }));
        }
    });

    // 等待关闭
    info!("All services started, waiting for requests...");
    match toplevel
        .catch_signals()
        .handle_shutdown_requests(tokio::time::Duration::from_secs(args.shutdown_timeout))
        .await
    {
        Ok(_) => {
            info!("Application gracefully shut down");
            Ok(())
        }
        Err(e) => {
            error!("Application shutdown error: {}", e);
            process::exit(1);
        }
    }
}

// 应用组件
struct AppComponents {
    // 管理服务
    admin_server: AdminServer,
    // 转发服务列表
    forward_servers: Vec<ForwardServer>,
}

// 创建应用组件
async fn create_components(config: Config) -> Result<AppComponents, AppError> {
    // 创建上游管理器
    let upstream_manager =
        match UpstreamManager::new(config.upstreams, config.upstream_groups).await {
            Ok(manager) => {
                info!("Upstream manager initialized successfully");
                Arc::new(manager)
            }
            Err(e) => {
                error!("Failed to initialize upstream manager: {}", e);
                return Err(e);
            }
        };

    // 创建管理服务
    let admin_addr = format!(
        "{}:{}",
        config.http_server.admin.address, config.http_server.admin.port
    )
    .parse()
    .map_err(|e| AppError::Config(format!("Invalid admin server address: {}", e)))?;
    let admin_server = AdminServer::new(admin_addr);
    info!("Admin server initialized successfully: {}", admin_addr);

    // 创建转发服务
    let mut forward_servers = Vec::with_capacity(config.http_server.forwards.len());
    for forward_config in config.http_server.forwards {
        match ForwardServer::new(forward_config.clone(), upstream_manager.clone()) {
            Ok(server) => {
                info!(
                    "Forwarding service {} initialized successfully",
                    forward_config.name
                );
                forward_servers.push(server);
            }
            Err(e) => {
                error!(
                    "Failed to initialize forwarding service {}: {}",
                    forward_config.name, e
                );
                return Err(e);
            }
        }
    }

    // 返回应用组件
    Ok(AppComponents {
        admin_server,
        forward_servers,
    })
}
