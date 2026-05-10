//! Server 命令行参数模块

use clap::Parser;

/// DAPRS Server - DN42 AutoPeering Server
#[derive(Parser, Debug)]
#[command(name = "server")]
#[command(about = "DAPRS Server - DN42 AutoPeering Server", long_about = None)]
pub struct Cli {
    /// 配置文件路径
    #[arg(short = 'c', long = "config", default_value = "server.toml")]
    pub config: String,
}
