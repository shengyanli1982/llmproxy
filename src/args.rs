use crate::r#const::shutdown_timeout;
use clap::{ArgAction, Parser};
use std::path::PathBuf;

// LLMProxy - 大模型代理服务
#[derive(Parser, Debug, Clone)]
#[command(
    name = "llmproxyd",
    author,
    version,
    about = "An intelligent load balancer with smart scheduling that unifies diverse LLMs (Public/Private Cloud, vLLM, Ollama), \nenabling seamless multi-cloud and hybrid-cloud adoption little client-side code modifications.\n\n\
             Key Features:\n\
             - Unified LLM Access & High Availability: Manages multiple LLM services (public/private APIs, vLLM/Ollama) with intelligent routing and failover.\n\
             - LLM-Optimized Load Balancing: Supports multiple strategies (round-robin, weighted, response-time aware, random, failover) specifically tuned for LLM workloads.\n\
             - Powerful Fault Tolerance: Built-in circuit breaker pattern isolates failing upstream services to prevent cascading failures.\n\
             - Seamless Scaling & Cost Control: Easily add or reduce upstream LLM services and prioritize resources through flexible load balancing.\n\
             - Simplified Integration: Provides a unified API entry point while shielding backend differences with minimal client code changes.\n\
             - Enhanced Observability: Detailed Prometheus metrics for real-time monitoring of LLM service calls and proxy performance.\n\
             - Fine-grained Traffic Control: Configure rate limits and QoS policies to protect backend LLM services.\n\
             - Enterprise-grade Security: Supports various authentication methods and secure connection management.\n\n\
             Author: shengyanli1982\n\
             Email: shengyanlee36@gmail.com\n\
             GitHub: https://github.com/shengyanli1982"
)]
pub struct Args {
    // 配置文件路径
    #[clap(
        short,
        long,
        value_name = "FILE",
        default_value = "config.yaml",
        help = "Path to the configuration file"
    )]
    pub config: PathBuf,

    // 是否开启调试模式
    #[clap(
        short, 
        long, 
        action = ArgAction::SetTrue, 
        help = "Enable debug mode"
    )]
    pub debug: bool,

    // 是否仅测试配置文件
    #[clap(
        short = 't', 
        long = "test", 
        action = ArgAction::SetTrue, 
        help = "Test configuration file for validity and exit"
    )]
    pub test_config: bool,

    // 优雅关闭超时时间（秒）
    #[clap(
        long = "shutdown-timeout", 
        value_name = "SECONDS", 
        default_value_t = shutdown_timeout::DEFAULT, 
        help = "Maximum time in seconds to wait for complete shutdown"
    )]
    pub shutdown_timeout: u64,
}

impl Args {
    // 解析命令行参数
    pub fn parse_args() -> Self {
        Self::parse()
    }

    // 验证参数
    pub fn validation(&self) -> Result<(), String> {
        // 验证关闭超时时间
        if self.shutdown_timeout < shutdown_timeout::MIN
            || self.shutdown_timeout > shutdown_timeout::MAX
        {
            return Err(format!(
                "Shutdown timeout must be between {} and {} seconds",
                shutdown_timeout::MIN,
                shutdown_timeout::MAX
            ));
        }

        Ok(())
    }
}
