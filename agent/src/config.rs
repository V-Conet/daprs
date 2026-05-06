use serde::{Deserialize, Serialize};
use shared::{Dn42Config, NetConfig};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub agent: AgentConfig,
}
/// Configuration for the DAPRS agent.
/// example:
/// ```toml
/// [agent]
/// # 协议版本
/// version = 1
/// # 绑定地址
/// bind = "0.0.0.0:9813"
/// # 是否开放对外peering
/// is_open = true
/// # 是否需要人工验证
/// is_verify = false
/// # 额外信息
/// extra_msg = "hello world"

/// [agent.net]
/// ## 此节点公网环境
/// # 是否支持 IPv4
/// ipv4 = true
/// # 是否支持 IPv6
/// ipv6 = true
/// # 是否支持 NAT
/// accept_nat = true
/// # 是否允许中国大陆 Peering
/// cn = false

/// [agent.dn42]
/// ## 此节点部署的 DN42 相关信息
/// asn = 4242423322
/// ipv4 = "DN42_IPv4_ADDRESS"
/// ipv6 = "DN42_IPv6_ADDRESS"
/// lla = "DN42_LLA_ADDRESS"
/// wgkey = "DN42_WG_KEY"
/// ```
#[derive(Serialize, Deserialize, Clone)]
pub struct AgentConfig {
    pub version: u16,
    pub bind: String,
    #[serde(default)]
    pub api_token: String,
    pub is_open: bool,
    pub is_verify: bool,
    pub extra_msg: String,
    pub wg_path: String,
    pub bird_path: String,
    pub net: NetConfig,
    pub dn42: Dn42Config,
}

pub use shared::FrontendConfig;
