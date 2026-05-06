use serde::Serialize;

pub mod handler;
pub mod oauth;

#[derive(Serialize)]
pub struct NodeAgentConfig {
    pub address: String,
    pub online: bool,
    pub error: Option<String>,
    pub conf: FrontendAgentConfig,
}

pub type FrontendAgentConfig = shared::FrontendConfig;
