use serde::{Deserialize, Serialize};

// server/webui required

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: sled::Db,
}

/// Configuration for the DAPRS server.
/// example:
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
/// ```
#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    #[serde(default)]
    pub web: Option<WebConfig>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub version: u16,
    pub bind: String,
    pub api_token: String,
    pub alive: u32,
    pub servers: Vec<ServerInfo>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub address: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WebConfig {
    pub client_id: String,
    pub client_secret: String,
    pub oauth_provider: String,
    pub redirect_uri: String,
    #[serde(default)]
    pub frontend_origin: Option<String>,
}

#[allow(unused_imports)]
pub use shared::{Dn42Config, FrontendConfig, NetConfig};
