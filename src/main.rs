use llmproxy::{
    admin::AdminServer, args::Args, config::Config, error::AppError, server::ForwardServer,
    upstream::UpstreamManager,
};
use mimalloc::MiMalloc;
use std::{collections::HashMap, process, sync::Arc};
use tokio::sync::RwLock;
use tokio_graceful_shutdown::{IntoSubsystem, SubsystemBuilder, Toplevel};
use tracing::{error, info};

// 使用 mimalloc 分配器提高内存效率
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn init_logging(args: &Args) {
    let builder = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_line_number(false);

    // 如果启用调试模式，输出调试信息，否则只输出 info 及以上级别
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
    let components = match create_components(args.debug, config).await {
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
            info!("Application gracefully shutdown");
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
async fn create_components(debug: bool, config: Config) -> Result<AppComponents, AppError> {
    // 创建配置的共享引用，使用RwLock包装以支持动态更新
    let config_arc = Arc::new(RwLock::new(config));

    // 在一个读锁范围内获取所有配置，避免多次获取锁
    let (upstreams, upstream_groups, http_server_config) = {
        let config_guard = config_arc.read().await;
        let http_server = config_guard
            .http_server
            .clone()
            .ok_or_else(|| AppError::Config("http_server configuration is missing".to_string()))?;

        (
            config_guard.upstreams.clone(),
            config_guard.upstream_groups.clone(),
            http_server,
        )
    };

    // 创建上游管理器
    let upstream_manager: Arc<UpstreamManager> =
        match UpstreamManager::new(upstreams, upstream_groups).await {
            Ok(manager) => Arc::new(manager),
            Err(e) => {
                error!("Failed to initialize upstream manager: {}", e);
                return Err(e);
            }
        };

    // 创建转发服务
    let mut forward_servers = Vec::with_capacity(http_server_config.forwards.len());

    // 预先分配HashMap容量，减少重新分配
    let mut forward_states_map = HashMap::with_capacity(http_server_config.forwards.len());

    for forward_config in &http_server_config.forwards {
        // 使用克隆避免所有权转移
        match ForwardServer::new(forward_config.clone(), upstream_manager.clone()) {
            Ok(server) => {
                info!(
                    "Forwarding service {:?} initialized successfully",
                    forward_config.name
                );

                // 获取状态并直接插入HashMap（避免后续再克隆）
                forward_states_map.insert(forward_config.name.clone(), server.get_state().clone());
                forward_servers.push(server);
            }
            Err(e) => {
                error!(
                    "Failed to initialize forwarding service {:?}: {}",
                    forward_config.name, e
                );
                return Err(e);
            }
        }
    }

    // 只在所有状态收集完成后创建一次Arc
    let forward_states = Arc::new(forward_states_map);

    // 创建管理服务
    let admin_addr = format!(
        "{}:{}",
        http_server_config.admin.address, http_server_config.admin.port
    )
    .parse()
    .map_err(|e| AppError::Config(format!("Invalid admin server address: {}", e)))?;
    let admin_server = AdminServer::new(debug, admin_addr, config_arc.clone(), forward_states);
    info!("Admin server initialized successfully: {:?}", admin_addr);

    // 返回应用组件
    Ok(AppComponents {
        admin_server,
        forward_servers,
    })
}
