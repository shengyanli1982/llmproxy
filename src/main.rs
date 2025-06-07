use llmproxy::admin::AdminServer;
use llmproxy::args::Args;
use llmproxy::config::Config;
use llmproxy::error::AppError;
use llmproxy::manager::ServerManager;
use llmproxy::upstream::UpstreamManager;
use mimalloc::MiMalloc;
use std::process;
use std::sync::Arc;
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

    // 创建共享配置引用
    let shared_config = Arc::new(RwLock::new(config));

    // 创建应用组件
    let components = match create_components(Arc::clone(&shared_config)).await {
        Ok(components) => components,
        Err(e) => {
            error!("Failed to create application components: {}", e);
            process::exit(1);
        }
    };

    // 创建优雅关闭顶层管理器
    let toplevel = Toplevel::new(|s| async move {
        // 启动服务器管理子系统
        let server_manager = components.server_manager;
        s.start(SubsystemBuilder::new(
            "server_manager",
            move |s| async move { server_manager.run(s).await },
        ));

        // 启动管理服务子系统
        let admin_server = components.admin_server;
        s.start(SubsystemBuilder::new("admin_server", move |s| async move {
            admin_server.run(s).await
        }));
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
    // 服务器管理器
    server_manager: ServerManager,
}

// 创建应用组件
async fn create_components(shared_config: Arc<RwLock<Config>>) -> Result<AppComponents, AppError> {
    // 获取配置的只读视图并克隆必要数据，然后释放锁
    let (upstreams, upstream_groups, admin_address, admin_port) = {
        let config = shared_config.read().await;
        (
            config.upstreams.clone(),
            config.upstream_groups.clone(),
            config.http_server.admin.address.clone(),
            config.http_server.admin.port,
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

    // 创建服务器管理器
    let (server_sender, server_receiver) = ServerManager::create_channel();
    let server_manager = ServerManager::new(
        Arc::clone(&shared_config),
        upstream_manager.clone(),
        server_receiver,
    );
    info!("Server manager initialized successfully");

    // 创建管理服务
    let admin_addr = format!("{}:{}", admin_address, admin_port)
        .parse()
        .map_err(|e| AppError::Config(format!("Invalid admin server address: {}", e)))?;
    let admin_server = AdminServer::new(
        admin_addr,
        Arc::clone(&shared_config),
        server_sender.clone(),
    );
    info!("Admin server initialized successfully: {}", admin_addr);

    // 返回应用组件
    Ok(AppComponents {
        admin_server,
        server_manager,
    })
}
