use std::path::Path;
use std::sync::{Mutex, OnceLock};

use anyhow::Context;
use clap::Parser;
use serde::{Deserialize, Serialize};
use shared::AgentNode;

/// 全局配置：由 [`init_config`] 在启动时写入，命令处理器通过 [`config`] 读取。
static CONFIG: OnceLock<Mutex<Config>> = OnceLock::new();

/// 启动时写入全局配置（重复调用会被忽略）。
pub fn init_config(cfg: Config) {
    let _ = CONFIG.set(Mutex::new(cfg));
}

/// 获取全局配置的互斥引用。
pub fn config() -> &'static Mutex<Config> {
    CONFIG.get().expect("config not initialized")
}

/// DAPRS TgBot - DN42 Network Tools Bot
#[derive(Parser, Debug)]
#[command(name = "tgbot")]
#[command(about = "DAPRS TgBot - DN42 Network Tools Bot", long_about = None)]
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
    /// 其他设置
    pub settings: TgbotSettings,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentConfig {
    /// Agent 地址列表
    pub nodes: Vec<NodeConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TgbotSettings {
    pub start_msg: String,
    pub flap_url: Option<String>,
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
