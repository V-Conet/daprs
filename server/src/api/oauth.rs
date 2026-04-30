use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{
    Json,
    extract::{Query, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, TokenResponse, TokenUrl, basic::BasicClient,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{AppState, WebConfig};

const OAUTH_LOGIN_STATE_TREE: &str = "oauth_login_state";
const OAUTH_SESSION_TREE: &str = "oauth_sessions";
const LOGIN_STATE_TTL_SECS: u64 = 600;
const SESSION_TTL_SECS: u64 = 8 * 60 * 60;
const SESSION_COOKIE_NAME: &str = "daprs_session";

pub async fn login(State(state): State<AppState>) -> Result<Redirect, StatusCode> {
    let web = state
        .config
        .web
        .as_ref()
        .ok_or(StatusCode::NOT_IMPLEMENTED)?;
    let oauth = create_oauth_client(web).await?;

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let csrf = CsrfToken::new_random();
    let csrf_secret = csrf.secret().to_string();

    let (authorization_url, _) = oauth
        .client
        .authorize_url(|| csrf)
        .set_pkce_challenge(pkce_challenge)
        .url();

    let login_state = OAuthLoginState {
        pkce_verifier: pkce_verifier.secret().to_string(),
        created_at: now_unix_secs(),
    };

    persist_json(
        &state.db,
        OAUTH_LOGIN_STATE_TREE,
        &csrf_secret,
        &login_state,
    )?;

    Ok(Redirect::to(authorization_url.as_str()))
}

pub async fn login_callback(
    State(state): State<AppState>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Response, StatusCode> {
    let web = state
        .config
        .web
        .as_ref()
        .ok_or(StatusCode::NOT_IMPLEMENTED)?;
    let oauth = create_oauth_client(web).await?;

    if query.error.is_some() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let code = query.code.ok_or(StatusCode::BAD_REQUEST)?;
    let state_value = query.state.ok_or(StatusCode::BAD_REQUEST)?;
    let login_state = consume_login_state(&state, &state_value)?;

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let token = oauth
        .client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(PkceCodeVerifier::new(login_state.pkce_verifier))
        .request_async(&http_client)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let userinfo = fetch_userinfo(
        &http_client,
        &oauth.discovery,
        token.access_token().secret(),
    )
    .await?;

    let session_id = CsrfToken::new_random().secret().to_string();
    let now = now_unix_secs();
    let session = OAuthSession {
        userinfo,
        issued_at: now,
        expires_at: now + SESSION_TTL_SECS,
    };

    persist_json(&state.db, OAUTH_SESSION_TREE, &session_id, &session)?;

    let mut response = Redirect::to("/").into_response();
    let set_cookie = build_session_cookie(&session_id, web)?;
    response
        .headers_mut()
        .append(header::SET_COOKIE, set_cookie);
    Ok(response)
}

pub async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, StatusCode> {
    let session = require_session(&state, &headers)?;
    Ok(Json(MeResponse {
        issued_at: session.issued_at,
        expires_at: session.expires_at,
        userinfo: session.userinfo,
    }))
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    if let Some(session_id) = extract_cookie(&headers, SESSION_COOKIE_NAME) {
        let tree = state
            .db
            .open_tree(OAUTH_SESSION_TREE)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        tree.remove(session_id.as_bytes())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        tree.flush()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_static("daprs_session=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax"),
    );
    Ok(response)
}

pub async fn require_auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if state.config.web.is_none() {
        return Ok(next.run(request).await);
    }

    require_session(&state, request.headers())?;
    Ok(next.run(request).await)
}

async fn create_oauth_client(web: &WebConfig) -> Result<OAuthRuntime, StatusCode> {
    validate_required_web_config(web)?;

    let discovery_url = normalize_discovery_url(&web.oauth_provider);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let discovery = client
        .get(&discovery_url)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .error_for_status()
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .json::<OidcDiscoveryDocument>()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    validate_https_endpoint(&discovery.authorization_endpoint)?;
    validate_https_endpoint(&discovery.token_endpoint)?;
    if let Some(userinfo_endpoint) = &discovery.userinfo_endpoint {
        validate_https_endpoint(userinfo_endpoint)?;
    }

    let oauth_client = BasicClient::new(ClientId::new(web.client_id.clone()))
        .set_client_secret(ClientSecret::new(web.client_secret.clone()))
        .set_auth_uri(
            AuthUrl::new(discovery.authorization_endpoint.clone())
                .map_err(|_| StatusCode::BAD_REQUEST)?,
        )
        .set_token_uri(
            TokenUrl::new(discovery.token_endpoint.clone()).map_err(|_| StatusCode::BAD_REQUEST)?,
        )
        .set_redirect_uri(
            RedirectUrl::new(web.redirect_uri.clone()).map_err(|_| StatusCode::BAD_REQUEST)?,
        );

    Ok(OAuthRuntime {
        client: oauth_client,
        discovery,
    })
}

async fn fetch_userinfo(
    http_client: &reqwest::Client,
    discovery: &OidcDiscoveryDocument,
    access_token: &str,
) -> Result<Value, StatusCode> {
    let endpoint = discovery
        .userinfo_endpoint
        .as_ref()
        .ok_or(StatusCode::BAD_GATEWAY)?;

    let response = http_client
        .get(endpoint)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .error_for_status()
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    response
        .json::<Value>()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)
}

fn require_session(state: &AppState, headers: &HeaderMap) -> Result<OAuthSession, StatusCode> {
    let session_id =
        extract_cookie(headers, SESSION_COOKIE_NAME).ok_or(StatusCode::UNAUTHORIZED)?;
    let tree = state
        .db
        .open_tree(OAUTH_SESSION_TREE)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let value = tree
        .get(session_id.as_bytes())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let session: OAuthSession =
        serde_json::from_slice(&value).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if session.expires_at <= now_unix_secs() {
        tree.remove(session_id.as_bytes())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        tree.flush()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(session)
}

fn consume_login_state(state: &AppState, csrf_state: &str) -> Result<OAuthLoginState, StatusCode> {
    let tree = state
        .db
        .open_tree(OAUTH_LOGIN_STATE_TREE)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let value = tree
        .remove(csrf_state.as_bytes())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::BAD_REQUEST)?;
    tree.flush()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let login_state: OAuthLoginState =
        serde_json::from_slice(&value).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if now_unix_secs() > login_state.created_at + LOGIN_STATE_TTL_SECS {
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(login_state)
}

fn persist_json<T: Serialize>(
    db: &sled::Db,
    tree_name: &str,
    key: &str,
    value: &T,
) -> Result<(), StatusCode> {
    let tree = db
        .open_tree(tree_name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let encoded = serde_json::to_vec(value).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tree.insert(key.as_bytes(), encoded)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    tree.flush()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}

fn extract_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;
    for segment in cookies.split(';') {
        let trimmed = segment.trim();
        let (k, v) = trimmed.split_once('=')?;
        if k == name {
            return Some(v.to_string());
        }
    }
    None
}

fn build_session_cookie(session_id: &str, web: &WebConfig) -> Result<HeaderValue, StatusCode> {
    let secure = is_https_url(&web.redirect_uri)
        || web
            .frontend_origin
            .as_ref()
            .is_some_and(|origin| is_https_url(origin));
    let same_site_attr = if web.frontend_origin.is_some() && secure {
        "; SameSite=None"
    } else {
        "; SameSite=Lax"
    };
    let secure_attr = if secure { "; Secure" } else { "" };
    let cookie = format!(
        "{}={}; Path=/; Max-Age={}; HttpOnly{}{}{}",
        SESSION_COOKIE_NAME, session_id, SESSION_TTL_SECS, same_site_attr, secure_attr, ""
    );
    HeaderValue::from_str(&cookie).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn validate_required_web_config(web: &WebConfig) -> Result<(), StatusCode> {
    if web.client_id.trim().is_empty()
        || web.client_secret.trim().is_empty()
        || web.oauth_provider.trim().is_empty()
        || web.redirect_uri.trim().is_empty()
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    if !is_https_url(&web.oauth_provider) {
        return Err(StatusCode::BAD_REQUEST);
    }

    if !(is_https_url(&web.redirect_uri) || is_loopback_http_url(&web.redirect_uri)) {
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(())
}

fn normalize_discovery_url(provider: &str) -> String {
    provider.trim_end_matches('/').to_string()
}

fn validate_https_endpoint(url: &str) -> Result<(), StatusCode> {
    if is_https_url(url) {
        return Ok(());
    }
    Err(StatusCode::BAD_GATEWAY)
}

fn is_https_url(url: &str) -> bool {
    url.starts_with("https://")
}

fn is_loopback_http_url(url: &str) -> bool {
    url.starts_with("http://127.0.0.1") || url.starts_with("http://localhost")
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

struct OAuthRuntime {
    client: BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,
    discovery: OidcDiscoveryDocument,
}

#[derive(Deserialize, Clone)]
struct OidcDiscoveryDocument {
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: Option<String>,
}

#[derive(Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct OAuthLoginState {
    pkce_verifier: String,
    created_at: u64,
}

#[derive(Serialize, Deserialize, Clone)]
struct OAuthSession {
    userinfo: Value,
    issued_at: u64,
    expires_at: u64,
}

#[derive(Serialize)]
pub struct MeResponse {
    issued_at: u64,
    expires_at: u64,
    userinfo: Value,
}
