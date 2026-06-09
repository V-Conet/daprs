//! Server API 模块

pub mod handler;
pub mod oauth;

/// 前端所需 Agent 配置
pub type FrontendAgentConfig = shared::FrontendConfig;

/// 节点 Agent 配置响应
pub type NodeAgentConfig = shared::NodeAgentConfig;
