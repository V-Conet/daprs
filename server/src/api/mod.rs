//! Server API 模块
//!
//! 鉴权 → 仓储层 → agent_client → 仓储层 → 审计

pub mod handler;
pub mod oauth;

use serde::{Deserialize, Serialize};
use shared::PeeringPayload;

/// 前端所需 Agent 配置
pub type FrontendAgentConfig = shared::FrontendConfig;

/// 节点 Agent 配置响应
pub type NodeAgentConfig = shared::NodeAgentConfig;

/// 待审核 Peering 请求（API 响应类型）
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PendingRequest {
    /// 请求 ID
    pub id: String,
    /// 节点名称
    pub node: String,
    /// 请求者 ASN
    pub asn: u32,
    /// Peering 配置
    pub payload: PeeringPayload,
    /// 创建时间（unix 秒）
    pub created_at: u64,
}

/// 管理员 Peer 修改请求
#[derive(Deserialize)]
pub struct AdminPeerRequest {
    pub node: String,
    pub asn: u32,
    pub payload: PeeringPayload,
}

/// 管理员删除请求
#[derive(Deserialize)]
pub struct AdminDeleteRequest {
    pub node: String,
    pub asn: u32,
}
