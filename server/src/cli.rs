use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "server")]
#[command(about = "DAPRS Server Client")]

pub struct Cli {
    /// 配置文件路径
    #[arg(short = 'c', long = "config", default_value = "config.toml")]
    pub config: String,
}
