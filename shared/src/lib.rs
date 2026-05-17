//! DAPRS 共享模块
//!
//! 提供 Agent 和 Server 之间共享的数据结构、错误类型和工具函数。

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod rate_limiter;
pub mod validation;

// 错误类型
/// 应用程序统一错误类型
///
/// 使用 thiserror 自动实现 std::error::Error trait
/// 并提供 axum 的 IntoResponse 实现
#[derive(Debug, Error)]
pub enum AppError {
    /// 未授权（认证失败或 token 无效）
    #[error("Unauthorized")]
    Unauthorized,

    /// 请求参数错误
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// 内部服务器错误
    #[error("Internal error: {0}")]
    InternalError(String),

    /// 网关错误（无法连接到 Agent）
    #[error("Bad gateway")]
    BadGateway,

    /// 资源未找到
    #[error("Not found")]
    NotFound,

    /// 服务不可用（如节点不开放 peering）
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    /// 请求过于频繁（速率限制）
    #[error("Too many requests")]
    TooManyRequests,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::BadGateway => StatusCode::BAD_GATEWAY,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            AppError::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
        };

        // 内部错误信息不应泄露给客户端，只记录日志
        let message = match &self {
            AppError::InternalError(detail) => {
                tracing::error!("internal error: {detail}");
                "Internal server error".to_string()
            }
            other => other.to_string(),
        };

        let body = Json(serde_json::json!({
            "error": message
        }));

        (status, body).into_response()
    }
}

// 网络配置
/// 节点网络能力配置
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct NetConfig {
    /// 是否支持 IPv4
    pub ipv4: bool,
    /// 是否支持 IPv6
    pub ipv6: bool,
    /// 是否接受 NAT 环境
    pub accept_nat: bool,
    /// 是否允许中国大陆 Peering
    pub cn: bool,
}

// DN42 配置
/// DN42 相关配置信息
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Dn42Config {
    /// ASN 号码
    pub asn: u32,
    /// DN42 IPv4 地址
    #[serde(default)]
    pub ipv4: String,
    /// DN42 IPv6 地址
    #[serde(default)]
    pub ipv6: String,
    /// 链路本地地址
    #[serde(default)]
    pub lla: String,
    /// WireGuard 公钥
    #[serde(default)]
    pub wgkey: String,
    /// 公网 IPv4 地址
    #[serde(default)]
    pub ipv4_addr: String,
    /// 公网 IPv6 地址
    #[serde(default)]
    pub ipv6_addr: String,
}

// 前端配置
/// 供前端展示的配置信息
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FrontendConfig {
    /// 协议版本
    pub version: u16,
    /// 是否开放对外 peering
    pub is_open: bool,
    /// 是否需要人工验证
    pub is_verify: bool,
    /// 额外信息
    pub extra_msg: String,
    /// 网络能力配置
    #[serde(default)]
    pub net: NetConfig,
    /// DN42 配置
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

// Peering 请求
/// Peering 配置请求
///
/// 包含建立 WireGuard 隧道和 BGP 会话所需的所有信息。
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PeeringPayload {
    /// 是否 MultiHop
    pub is_mhp: bool,
    /// 是否 Extended NextHop
    pub is_nhp: bool,
    /// 本节点 DN42 IPv4
    pub v4: Option<String>,
    /// 本节点 DN42 IPv6
    pub v6: Option<String>,
    /// 链路本地地址
    pub lla: Option<String>,
    /// 是否优先使用链路本地地址
    pub is_prefer_lla: bool,
    /// 对端 WireGuard Endpoint
    pub endpoint: String,
    /// 对端 WireGuard 公钥
    pub pubkey: String,
    /// 自定义端口
    pub custom_port: Option<u16>,
    /// 预共享密钥（可选）
    pub psk: Option<String>,
    /// MTU
    pub mtu: Option<u16>,
}

/// 节点操作请求
#[derive(Deserialize, Serialize, Debug)]
pub struct NodeActionRequest<T> {
    /// 目标节点名称
    pub node: String,
    /// 操作载荷
    pub payload: T,
}

/// 删除 Peer 请求
#[derive(Deserialize, Serialize, Debug)]
pub struct RemoveRequest {
    /// 目标节点名称
    pub node: String,
}

// 命令类型
/// DNS 查询类型
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum QueryType {
    /// IPv4 地址
    #[default]
    A,
    /// IPv6 地址
    AAAA,
    /// 任意类型
    ANY,
    /// CNAME 记录
    CNAME,
    /// 邮件交换记录
    MX,
    /// 域名服务器
    NS,
    /// 指针记录（反向 DNS）
    PTR,
    /// 起始授权机构
    SOA,
    /// 服务记录
    SRV,
    /// 文本记录
    TXT,
    /// DNSSEC 公钥
    DNSKEY,
    /// 委派签名者
    DS,
    /// NSEC 记录
    NSEC,
    /// NSEC3 记录
    NSEC3,
    /// RRSIG 记录
    RRSIG,
}

impl QueryType {
    /// 获取 dig 命令使用的参数字符串
    pub fn as_dig_arg(&self) -> &'static str {
        match self {
            QueryType::A => "A",
            QueryType::AAAA => "AAAA",
            QueryType::ANY => "ANY",
            QueryType::CNAME => "CNAME",
            QueryType::MX => "MX",
            QueryType::NS => "NS",
            QueryType::PTR => "PTR",
            QueryType::SOA => "SOA",
            QueryType::SRV => "SRV",
            QueryType::TXT => "TXT",
            QueryType::DNSKEY => "DNSKEY",
            QueryType::DS => "DS",
            QueryType::NSEC => "NSEC",
            QueryType::NSEC3 => "NSEC3",
            QueryType::RRSIG => "RRSIG",
        }
    }
}

/// 支持的命令类型
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "op", content = "args")]
#[serde(rename_all = "lowercase")]
pub enum Cmd {
    /// Ping 命令
    ///
    /// 用法:
    /// ```text
    /// ping [-c count] [-s size] [-F] [-t timeout] [-i interval] [-4|-6] <target>
    /// ```
    Ping {
        /// 协议版本 (4 或 6)
        protocol: Option<u16>,
        /// 发送次数，默认 4
        count: Option<u16>,
        /// 数据包大小
        size: Option<u16>,
        /// 是否设置 DF 标志
        dfrag: Option<bool>,
        /// 超时时间（毫秒），默认 2000
        timeout: Option<u32>,
        /// 目标地址
        target: String,
    },

    /// Traceroute 命令
    ///
    /// 用法:
    /// ```text
    /// traceroute [-4|-6] <target>
    /// ```
    Traceroute {
        /// 协议版本 (4 或 6)
        protocol: Option<u16>,
        /// 目标地址
        target: String,
    },

    /// Dig 命令
    ///
    /// 用法:
    /// ```text
    /// dig <domain> {type} {@server}
    /// ```
    Dig {
        /// 查询类型
        #[serde(default)]
        qtype: QueryType,
        /// DNS 服务器（含端口）
        server: Option<String>,
        /// 目标域名
        target: String,
    },

    /// WireGuard show 命令
    WgShow {
        /// 接口名称
        interface: String,
    },

    /// Bird show protocol 命令
    BirdShow {
        /// 协议名称
        protocol: String,
    },

    /// TCP Ping 命令
    ///
    /// 使用 TCP 进行连通性测试
    /// 用法: tcping <host> <port> [-c count] [-t timeout]
    TcPing {
        /// 协议版本 (4 或 6)
        protocol: Option<u16>,
        /// 目标地址
        target: String,
        /// 目标端口
        port: u16,
        /// 发送次数，默认 5
        count: Option<u16>,
        /// 超时时间（秒），默认 3
        timeout: Option<u8>,
    },

    /// 查看路由表命令
    ///
    /// 用法: route <target> [-4|-6]
    Route {
        /// 协议版本 (4 或 6)
        protocol: Option<u16>,
        /// 目标地址（支持 CIDR）
        target: String,
    },

    /// 显示 AS Path 命令
    ///
    /// 用法: path <target> [-4|-6]
    Path {
        /// 协议版本 (4 或 6)
        protocol: Option<u16>,
        /// 目标地址
        target: String,
    },
}

/// 命令请求
#[derive(Serialize, Deserialize, Debug)]
pub struct CmdRequest {
    /// 命令
    pub cmd: Cmd,
}

// API 响应类型
/// 对端信息
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PeerInfo {
    /// 对端公钥
    pub pubkey: String,
    /// 对端 Endpoint
    pub endpoint: Option<String>,
    /// 对端 IPv4
    pub v4: Option<String>,
    /// 对端 IPv6
    pub v6: Option<String>,
}

/// WireGuard 配置
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WgConfig {
    /// 监听端口
    pub port: u16,
    /// MTU
    pub mtu: u16,
    /// 对端公钥
    pub pubkey: String,
    /// 预共享密钥
    pub psk: Option<String>,
    /// Endpoint
    pub endpoint: Option<String>,
    /// 对端 IPv4
    pub peer_v4: Option<String>,
    /// 对端 IPv6 (ULA 或 LLA)
    pub peer_v6: Option<String>,
}

/// Bird 配置
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BirdConfig {
    /// 是否使用 MP-BGP
    pub is_mhp: bool,
    /// 是否使用 Extended Next Hop
    pub is_nhp: bool,
    /// 会话类型描述
    pub session_type: String,
}

/// 原始命令输出
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RawCommandOutput {
    /// 执行的命令
    pub command: String,
    /// 命令输出
    pub output: String,
}

/// Peer 信息响应
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PeerInfoResponse {
    /// ASN 号码
    pub asn: u32,
    /// 接口是否在线
    pub interface_up: bool,
    /// wg show 输出（仅当接口在线时）
    pub wg_show: Option<RawCommandOutput>,
    /// birdc show protocol 输出（仅当有 peer 时）
    pub bird_protocols: Vec<RawCommandOutput>,
    /// 本节点 DN42 IPv4
    pub my_v4: String,
    /// 本节点 DN42 IPv6
    pub my_v6: String,
    /// 本节点链路本地地址
    pub my_lla: String,
    /// 本节点 WireGuard 公钥
    pub my_pubkey: String,
    /// 对端配置信息
    pub peer: Option<PeerInfo>,
    /// WireGuard 配置详情（用于修改表单）
    pub wg: Option<WgConfig>,
    /// Bird 配置详情
    pub bird: Option<BirdConfig>,
}
