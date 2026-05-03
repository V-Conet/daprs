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


#[derive(Deserialize, Serialize)]
pub struct PeeringPayload {
    /// MultiHop
    pub is_mhp: bool,
    /// Extended NextHop
    pub is_nhp: bool,
    /// 路由策略
    pub policy: RoutingPolicy,
    /// 本节点 DN42 IPv4
    pub v4: Option<String>,
    /// 本节点 DN42 IPv6
    pub v6: Option<String>,
    /// 本节点链路本地地址
    pub lla: Option<String>,
    /// 是否优先使用链路本地地址
    pub is_prefer_lla: bool,
    /// 对端地址
    pub endpoint: String,
    /// 对端公钥
    pub pubkey: String,
    /// 开放给对端的端口
    pub custom_port: Option<u16>,
    /// 预共享密钥
    pub psk: Option<String>,
    /// MTU
    pub mtu: Option<u16>,
}

#[derive(Deserialize, Serialize)]
pub struct NodeActionRequest<T> {
    /// 目标节点名称
    pub node: String,
    /// Payload from frontend
    pub payload: T,
}

#[derive(Deserialize, Serialize)]
pub struct RemoveRequest {
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
    #[serde(default)]
    pub net: NetConfig,
    #[serde(default)]
    pub dn42: Dn42Config,
}

impl Default for FrontendAgentConfig {
    fn default() -> Self {
        Self {
            version: 1,
            is_open: false,
            is_verify: false,
            extra_msg: "agent unavailable".into(),
            net: NetConfig::default(),
            dn42: Dn42Config::default(),
        }
    }
}
