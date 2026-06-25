use std::path::Path;

use anyhow::Context;
use clap::Parser;
use serde::{Deserialize, Serialize};
use shared::AgentNode;

/// DAPRS TgBot - DN42 Network Tools Telegram Bot
#[derive(Parser, Debug)]
#[command(name = "tgbot")]
#[command(about = "DAPRS TgBot - DN42 Network Tools Telegram Bot", long_about = None)]
pub struct Cli {
    /// 配置文件路径
    #[arg(short = 'c', long = "config", default_value = "tgbot.toml")]
    pub config: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub tgbot: TgbotConfig,
    pub agent: AgentConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TgbotConfig {
    /// Agent API Token
    pub api_token: String,
    /// 命令超时时间（秒）
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentConfig {
    /// Agent 地址列表
    pub nodes: Vec<NodeConfig>,
}

pub type NodeConfig = AgentNode;

fn default_timeout() -> u64 {
    30
}

/// 从 TOML 文件加载配置
pub fn load_config<P: AsRef<Path>>(path: P) -> anyhow::Result<Config> {
    let content = std::fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to load config: {}", path.as_ref().display()))?;
    Ok(config)
}
