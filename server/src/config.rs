//! Server 配置模块

use serde::{Deserialize, Serialize};
use shared::rate_limiter::RateLimitConfig;

/// 应用全局状态
#[derive(Clone)]
pub struct AppState {
    /// 配置
    pub config: Config,
    /// 数据库
    pub db: sled::Db,
}

/// Server 完整配置
///
/// # 配置示例
///
/// ```toml
/// [server]
/// version = 1
/// bind = "0.0.0.0:8080"
/// api_token = "TOKEN"
/// alive = 60
/// servers = [
///     { name = "server1", address = "IP_ADDRESS:PORT" },
///     { name = "server2", address = "IP_ADDRESS:PORT" },
/// ]
///
/// [server.rate_limit.auth]
/// window_secs = 10
/// max_requests = 5

/// [server.rate_limit.api]
/// window_secs = 60
/// max_requests = 100
///
/// [web]
/// client_id = "KEY"
/// client_secret = "SECRET"
/// oauth_provider = "https://auth.example.com/.well-known/openid-configuration"
/// redirect_uri = "http://localhost:8080/api/login/callback"
/// frontend_origin = "http://localhost:5173"
/// ```
#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    /// Server 配置
    pub server: ServerConfig,
    /// Web 配置
    pub web: WebConfig,
}

/// Server 配置详情
#[derive(Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    /// 协议版本
    pub version: u16,
    /// HTTP 服务绑定地址
    pub bind: String,
    /// Server/Agent 认证 token
    pub api_token: String,
    /// Keepalive 时间
    pub alive: u32,
    /// Agent 服务器列表
    pub servers: Vec<ServerInfo>,
    /// 速率限制配置（可选）
    #[serde(default)]
    pub rate_limit: Option<RateLimitConfig>,
}

/// Agent 服务器信息
#[derive(Serialize, Deserialize, Clone)]
pub struct ServerInfo {
    /// 服务器名称
    pub name: String,
    /// 服务器地址（IP:PORT）
    pub address: String,
}

/// Web 前端配置
#[derive(Serialize, Deserialize, Clone)]
pub struct WebConfig {
    /// OAuth 客户端 ID
    pub client_id: String,
    /// OAuth 客户端密钥
    pub client_secret: String,
    /// OAuth Provider 发现地址
    pub oauth_provider: String,
    /// OAuth 回调地址
    pub redirect_uri: String,
    /// 前端源地址（用于 CORS）
    #[serde(default)]
    pub frontend_origin: Option<String>,
}
