use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::config::Config;

pub async fn auth_middleware(
    State(config): State<Config>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if require_token(&request.headers(), &config).unwrap_or(false) {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

// TODO: refactor later
fn require_token(headers: &HeaderMap, config: &Config) -> Result<bool, StatusCode> {
    match headers.get("x-api-token") {
        Some(token) => Ok(*token == config.agent.api_token),
        None => Err(StatusCode::UNAUTHORIZED),
    }
}
