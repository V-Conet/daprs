//! Agent API 模块

use axum::{Json, extract::State, http::HeaderMap};

use crate::config::{Config, FrontendConfig};
use crate::utils::verify_token;
use shared::AppError;

/// 获取节点配置信息
pub async fn get_config(
    headers: HeaderMap,
    State(config): State<Config>,
) -> Result<Json<FrontendConfig>, AppError> {
    verify_token(&headers, &config)?;

    Ok(Json(FrontendConfig {
        version: config.agent.version,
        is_open: config.agent.is_open,
        is_verify: config.agent.is_verify,
        extra_msg: config.agent.extra_msg,
        net: config.agent.net,
        dn42: config.agent.dn42,
    }))
}
