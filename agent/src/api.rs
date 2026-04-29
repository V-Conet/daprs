use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};

use crate::config::{Config, FrontendConfig};

pub async fn get_config(
    headers: HeaderMap,
    State(config): State<Config>,
) -> Result<Json<FrontendConfig>, axum::http::StatusCode> {
    require_api_token(&headers, &config)?;

    Ok(Json(FrontendConfig {
        version: config.agent.version,
        is_open: config.agent.is_open,
        is_verify: config.agent.is_verify,
        extra_msg: config.agent.extra_msg,
        net: config.agent.net,
        dn42: config.agent.dn42,
    }))
}

pub fn require_api_token(headers: &HeaderMap, config: &Config) -> Result<(), StatusCode> {
    if config.agent.api_token.is_empty() {
        return Ok(());
    }

    let header_value = headers
        .get("x-api-token")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if header_value != config.agent.api_token {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(())
}
