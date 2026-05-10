//! Server API 模块

use serde::Serialize;

pub mod handler;
pub mod oauth;

/// 前端所需 Agent 配置
pub type FrontendAgentConfig = shared::FrontendConfig;

// 响应类型
/// 节点 Agent 配置响应
#[derive(Serialize)]
pub struct NodeAgentConfig {
    /// Agent 地址
    pub address: String,
    /// 是否在线
    pub online: bool,
    /// 错误信息
    pub error: Option<String>,
    /// Agent 配置
    pub conf: FrontendAgentConfig,
}
