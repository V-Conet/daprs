use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "agent")]
#[command(about = "DAPRS Agent Client")]

pub struct Cli {
    /// 配置文件路径
    #[arg(short = 'c', long = "config", default_value = "agent.toml")]
    pub config: String,
}
