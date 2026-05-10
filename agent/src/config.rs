//! Agent 配置模块
//!
//! 定义 Agent 的配置结构体。

use serde::{Deserialize, Serialize};
use shared::{Dn42Config, NetConfig};

// 配置结构体
/// Agent 完整配置
#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub agent: AgentConfig,
}

/// Agent 配置详情
#[derive(Serialize, Deserialize, Clone)]
pub struct AgentConfig {
    /// 协议版本
    pub version: u16,
    /// HTTP 服务绑定地址
    pub bind: String,
    /// Server/Agent 认证 token（为空则跳过认证）
    #[serde(default)]
    pub api_token: String,
    /// 是否开放对外 peering
    pub is_open: bool,
    /// 是否需要人工验证
    pub is_verify: bool,
    /// 额外信息/公告
    pub extra_msg: String,
    /// WireGuard 配置文件存放路径
    pub wg_path: String,
    /// Bird 配置文件存放路径
    pub bird_path: String,
    /// 命令执行超时时间（秒），默认 15
    #[serde(default)]
    pub timeout: Option<u64>,
    /// 网络能力配置
    pub net: NetConfig,
    /// DN42 配置
    pub dn42: Dn42Config,
    /// 速率限制配置（可选）
    #[serde(default)]
    pub rate_limit: Option<shared::rate_limiter::RateLimitConfig>,
}

pub use shared::FrontendConfig;
