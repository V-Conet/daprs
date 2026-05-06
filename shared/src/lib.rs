use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct NetConfig {
    pub ipv4: bool,
    pub ipv6: bool,
    pub accept_nat: bool,
    pub cn: bool,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Dn42Config {
    pub asn: u32,
    #[serde(default)]
    pub ipv4: String,
    #[serde(default)]
    pub ipv6: String,
    #[serde(default)]
    pub lla: String,
    #[serde(default)]
    pub wgkey: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FrontendConfig {
    pub version: u16,
    pub is_open: bool,
    pub is_verify: bool,
    pub extra_msg: String,
    #[serde(default)]
    pub net: NetConfig,
    #[serde(default)]
    pub dn42: Dn42Config,
}

impl Default for FrontendConfig {
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

#[derive(Deserialize, Serialize)]
pub struct PeeringPayload {
    /// MultiHop
    pub is_mhp: bool,
    /// Extended NextHop
    pub is_nhp: bool,
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
