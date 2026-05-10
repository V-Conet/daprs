//! Agent 命令行参数模块

use clap::Parser;

/// DAPRS Agent - DN42 AutoPeering Agent
#[derive(Parser, Debug)]
#[command(name = "agent")]
#[command(about = "DAPRS Agent - DN42 AutoPeering Agent", long_about = None)]
pub struct Cli {
    /// 配置文件路径
    #[arg(short = 'c', long = "config", default_value = "agent.toml")]
    pub config: String,
}
