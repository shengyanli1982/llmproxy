use crate::r#const::shutdown_timeout;
use clap::{ArgAction, Parser};
use std::path::PathBuf;

/// LLMProxy - 大模型代理服务
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
pub struct Args {
    /// 配置文件路径
    #[clap(short, long, value_name = "FILE", default_value = "config.yaml")]
    pub config: PathBuf,

    /// 是否开启调试模式
    #[clap(short, long, action = ArgAction::SetTrue)]
    pub debug: bool,

    /// 是否仅测试配置文件
    #[clap(short = 't', long, action = ArgAction::SetTrue)]
    pub test_config: bool,

    /// 优雅关闭超时时间（秒）
    #[clap(long, value_name = "SECONDS", default_value_t = shutdown_timeout::DEFAULT)]
    pub shutdown_timeout: u64,
}

impl Args {
    /// 解析命令行参数
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// 验证参数
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
