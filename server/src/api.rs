use serde::{Deserialize, Serialize};

use crate::config::{Dn42Config, NetConfig};

pub mod handler;
pub mod oauth;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum RoutingPolicy {
    /// 接收所有有效路由，导出所有有效路由。
    FullTable,
    /// 接收所有有效路由，只导出本地自有路由。
    Transit,
    /// 只接收 AS_PATH = 1 的路由，导出本地自有路由和直接下游路由。
    PeeringOnly,
    /// 只接收 AS_PATH = 1 的路由，导出所有有效路由。
    Downstream,
}

/// 前端发送的peering请求
/// TODO: 其他参数和判断
#[derive(Deserialize, Serialize)]
pub struct PeeringPayload {
    ///  是否启用 Multi-Hop
    pub is_mhp: bool,
    /// 是否启用 NextHop
    pub is_nhp: bool,
    /// 路由策略
    pub policy: RoutingPolicy,
}

#[derive(Deserialize, Serialize)]
pub struct NodeActionRequest<T> {
    pub node: String,
    pub payload: T,
}

#[derive(Serialize, Deserialize)]
/// 交给agent处理的peering请求
pub struct PeeringRequest {
    /// 格式化后的peering请求字符串，包含完整 wireguard/bird peer 配置
    pub wgconfig: String,
    pub birdconfig: String,
}

// TODO: 这个东西并不是最后的设计，有待商榷
#[derive(Deserialize, Serialize)]
pub struct PeerInfo {
    pub asn: u32,
    pub node: String,
}

#[derive(Serialize)]
pub struct NodeAgentConfig {
    pub address: String,
    pub online: bool,
    pub error: Option<String>,
    pub conf: FrontendAgentConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FrontendAgentConfig {
    pub version: u16,
    pub is_open: bool,
    pub is_verify: bool,
    pub extra_msg: String,
    pub net: NetConfig,
    pub dn42: Dn42Config,
}

// for create/modify peer, return the full wg/bird config
// this method is under consideration, may be changed if we find a better way to handle peering config
#[derive(Serialize, Deserialize)]
pub struct WbConfig {
    pub wg_config: String,
    pub bird_config: String,
}
