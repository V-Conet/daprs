use serde::{Deserialize, Serialize};

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
    #[serde(default = "default_wg_config_path")]
    pub wg_config_path: String,
    #[serde(default = "default_bird_config_path")]
    pub bird_config_path: String,
    pub net: NetConfig,
    pub dn42: Dn42Config,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct NetConfig {
    pub ipv4: bool,
    pub ipv6: bool,
    pub accept_nat: bool,
    pub cn: bool,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Dn42Config {
    pub asn: u32,
    pub ipv4: String,
    pub ipv6: String,
    pub lla: String,
    pub wgkey: String,
}

// for frontend
#[derive(Serialize, Deserialize, Clone)]
pub struct FrontendConfig {
    pub version: u16,
    pub is_open: bool,
    pub is_verify: bool,
    pub extra_msg: String,
    pub net: NetConfig,
    pub dn42: Dn42Config,
}
// FIXME: the path shouldnt be here, it is defined in config.toml,
// write all conf to PATH_TO_CONFIG_WG/peerasn.conf for example
fn default_wg_config_path() -> String {
    "./data/wireguard-peer.conf".to_string()
}

fn default_bird_config_path() -> String {
    "./data/bird-peer.conf".to_string()
}
