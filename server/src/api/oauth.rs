//! OAuth 认证模块
//!
//! 处理 OIDC 认证流程
//!
//! 支持的 OIDC Provider:
//! - Kioubit, scope: dn42
//! - iEdon

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
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
    basic::BasicClient,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{AppState, OidcProvider, WebConfig};
use shared::AppError;

const LOGIN_STATE_TREE: &str = "oauth_login_state";
const SESSION_TREE: &str = "oauth_sessions";
const LOGIN_STATE_TTL_SECS: u64 = 600;
const SESSION_TTL_SECS: u64 = 8 * 60 * 60;
const SESSION_COOKIE: &str = "daprs_session";

// OAuth 客户端类型
type OAuthClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

// API Handlers
/// 发起 OAuth 登录
///
/// 重定向到 OAuth Provider 的授权页面。
pub async fn login(State(state): State<AppState>) -> Result<Redirect, AppError> {
    let (client, _) = create_oauth_client(&state.config.web).await?;
    let provider = state.config.web.provider;

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let csrf = CsrfToken::new_random();
    let csrf_secret = csrf.secret().to_string();

    let mut auth_request = client
        .authorize_url(|| csrf)
        .set_pkce_challenge(pkce_challenge)
        .add_scope(Scope::new("openid".to_string()));

    // Kioubit 需要 dn42 scope 来获取 ASN
    if matches!(provider, OidcProvider::Kioubit) {
        auth_request = auth_request.add_scope(Scope::new("dn42".to_string()));
    }

    let (authorization_url, _) = auth_request.url();

    let login_state = OAuthLoginState {
        pkce_verifier: pkce_verifier.secret().to_string(),
        created_at: now_unix_secs(),
    };

    persist_json(&state.db, LOGIN_STATE_TREE, &csrf_secret, &login_state)?;

    Ok(Redirect::to(authorization_url.as_str()))
}

/// OAuth 回调
///
/// 处理 OAuth Provider 的回调，完成登录流程。
pub async fn login_callback(
    State(state): State<AppState>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Response, AppError> {
    let web = &state.config.web;
    let (client, discovery) = create_oauth_client(web).await?;

    if query.error.is_some() {
        return Err(AppError::Unauthorized);
    }

    let code = query
        .code
        .ok_or(AppError::BadRequest("missing code".into()))?;
    let state_value = query
        .state
        .ok_or(AppError::BadRequest("missing state".into()))?;
    let login_state = consume_login_state(&state, &state_value)?;

    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::InternalError(format!("http client error: {e}")))?;

    let token = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(PkceCodeVerifier::new(login_state.pkce_verifier))
        .request_async(&http)
        .await
        .map_err(|e| AppError::InternalError(format!("token exchange error: {e}")))?;

    let userinfo = fetch_userinfo(&http, &discovery, token.access_token().secret()).await?;

    // 提取 ASN
    let asn = extract_asn(&userinfo)
        .ok_or_else(|| AppError::BadRequest("cannot extract ASN from userinfo".into()))?;

    // 将 ASN 存储到 userinfo 中，确保后续可以提取
    let mut userinfo_with_asn = userinfo;
    if let Some(obj) = userinfo_with_asn.as_object_mut() {
        obj.insert("_asn".to_string(), Value::Number(asn.into()));
    }

    let session_id = CsrfToken::new_random().secret().to_string();
    let now = now_unix_secs();
    let is_admin = state.config.server.admins.contains(&asn);
    let session = OAuthSession {
        userinfo: userinfo_with_asn,
        asn,
        is_admin,
        issued_at: now,
        expires_at: now + SESSION_TTL_SECS,
    };

    persist_json(&state.db, SESSION_TREE, &session_id, &session)?;

    // 登录成功后重定向到前端地址
    let redirect_url = web.frontend_origin.as_deref().unwrap_or("/");
    let mut response = Redirect::to(redirect_url).into_response();
    response
        .headers_mut()
        .append(header::SET_COOKIE, build_session_cookie(&session_id, web)?);
    Ok(response)
}

/// 获取当前用户信息
pub async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, AppError> {
    let session = require_session(&state, &headers)?;
    Ok(Json(MeResponse {
        issued_at: session.issued_at,
        expires_at: session.expires_at,
        userinfo: session.userinfo,
    }))
}

/// 退出登录
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    if let Some(session_id) = extract_cookie(&headers, SESSION_COOKIE) {
        let tree = state
            .db
            .open_tree(SESSION_TREE)
            .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;
        tree.remove(session_id.as_bytes())
            .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;
        tree.flush()
            .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;
    }

    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        build_clear_session_cookie(&state.config.web)?,
    );
    Ok(response)
}

/// 认证中间件
///
/// 检查请求是否已通过 OAuth 认证
pub async fn require_auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    require_session(&state, request.headers())?;
    Ok(next.run(request).await)
}

// Helper Functions
/// 从 Session 获取 ASN
pub(crate) fn get_session_asn(session: &OAuthSession) -> u32 {
    session.asn
}

/// 检查 Session 是否为管理员
pub(crate) fn is_session_admin(session: &OAuthSession) -> bool {
    session.is_admin
}

/// 验证是否为管理员
///
/// 每次请求都重新检查配置中的管理员列表，确保权限变更立即生效
pub(crate) fn require_admin(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<OAuthSession, AppError> {
    let session = require_session(state, headers)?;
    // 每次请求都重新检查，确保配置变更立即生效
    if !state.config.server.admins.contains(&session.asn) {
        return Err(AppError::Unauthorized);
    }
    Ok(session)
}

/// 从 userinfo 提取 ASN
fn extract_asn(userinfo: &Value) -> Option<u32> {
    userinfo
        .get("dn42")
        .and_then(|claim| claim.get("asn"))
        .and_then(|asn_val| asn_val.as_u64())
        .and_then(|n| u32::try_from(n).ok())
}

/// 验证 Session
pub(crate) fn require_session(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<OAuthSession, AppError> {
    let session_id = extract_cookie(headers, SESSION_COOKIE).ok_or(AppError::Unauthorized)?;
    let tree = state
        .db
        .open_tree(SESSION_TREE)
        .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;

    let value = tree
        .get(session_id.as_bytes())
        .map_err(|e| AppError::InternalError(format!("db error: {e}")))?
        .ok_or(AppError::Unauthorized)?;
    let session: OAuthSession = serde_json::from_slice(&value)
        .map_err(|e| AppError::InternalError(format!("json error: {e}")))?;

    if session.expires_at <= now_unix_secs() {
        tree.remove(session_id.as_bytes())
            .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;
        tree.flush()
            .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;
        return Err(AppError::Unauthorized);
    }

    Ok(session)
}

/// 持久化 JSON 数据
pub(crate) fn persist_json<T: Serialize>(
    db: &sled::Db,
    tree_name: &str,
    key: &str,
    value: &T,
) -> Result<(), AppError> {
    let tree = db
        .open_tree(tree_name)
        .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;
    let encoded = serde_json::to_vec(value)
        .map_err(|e| AppError::InternalError(format!("json error: {e}")))?;

    tree.insert(key.as_bytes(), encoded)
        .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;
    tree.flush()
        .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;
    Ok(())
}

/// 创建 OAuth 客户端
async fn create_oauth_client(web: &WebConfig) -> Result<(OAuthClient, OidcDiscovery), AppError> {
    // 验证基本配置
    if web.client_id.trim().is_empty() || web.client_secret.trim().is_empty() {
        return Err(AppError::BadRequest(
            "invalid OAuth config: missing client_id or client_secret".into(),
        ));
    }
    if web.redirect_uri.trim().is_empty()
        || !(web.redirect_uri.starts_with("http://") || web.redirect_uri.starts_with("https://"))
    {
        return Err(AppError::BadRequest(
            "invalid OAuth config: invalid redirect_uri".into(),
        ));
    }

    let discovery_url = web.discovery_url();

    // 获取 discovery 文档
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::InternalError(format!("http client error: {e}")))?;

    let discovery: OidcDiscovery = http
        .get(discovery_url)
        .send()
        .await
        .map_err(|e| AppError::InternalError(format!("discovery error: {e}")))?
        .error_for_status()
        .map_err(|e| AppError::InternalError(format!("discovery error: {e}")))?
        .json()
        .await
        .map_err(|e| AppError::InternalError(format!("discovery parse error: {e}")))?;

    // 验证 discovery 端点
    if !discovery.authorization_endpoint.starts_with("https://")
        || !discovery.token_endpoint.starts_with("https://")
        || discovery
            .userinfo_endpoint
            .as_ref()
            .is_some_and(|u| !u.starts_with("https://"))
    {
        return Err(AppError::BadRequest("invalid discovery endpoints".into()));
    }

    // 创建 OAuth 客户端
    let client = BasicClient::new(ClientId::new(web.client_id.clone()))
        .set_client_secret(ClientSecret::new(web.client_secret.clone()))
        .set_auth_uri(
            AuthUrl::new(discovery.authorization_endpoint.clone())
                .map_err(|e| AppError::InternalError(format!("auth url error: {e}")))?,
        )
        .set_token_uri(
            TokenUrl::new(discovery.token_endpoint.clone())
                .map_err(|e| AppError::InternalError(format!("token url error: {e}")))?,
        )
        .set_redirect_uri(
            RedirectUrl::new(web.redirect_uri.clone())
                .map_err(|e| AppError::InternalError(format!("redirect url error: {e}")))?,
        );

    Ok((client, discovery))
}

/// 获取用户信息
async fn fetch_userinfo(
    http: &reqwest::Client,
    discovery: &OidcDiscovery,
    access_token: &str,
) -> Result<Value, AppError> {
    let endpoint = discovery
        .userinfo_endpoint
        .as_ref()
        .ok_or(AppError::BadRequest(
            "userinfo endpoint not configured".into(),
        ))?;

    let response = http
        .get(endpoint)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| AppError::InternalError(format!("userinfo request error: {e}")))?
        .error_for_status()
        .map_err(|e| AppError::InternalError(format!("userinfo error: {e}")))?;

    response
        .json::<Value>()
        .await
        .map_err(|e| AppError::InternalError(format!("userinfo parse error: {e}")))
}

/// 消费登录状态
fn consume_login_state(state: &AppState, csrf_state: &str) -> Result<OAuthLoginState, AppError> {
    let tree = state
        .db
        .open_tree(LOGIN_STATE_TREE)
        .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;

    let value = tree
        .remove(csrf_state.as_bytes())
        .map_err(|e| AppError::InternalError(format!("db error: {e}")))?
        .ok_or(AppError::BadRequest("invalid or expired state".into()))?;
    tree.flush()
        .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;

    let login_state: OAuthLoginState = serde_json::from_slice(&value)
        .map_err(|e| AppError::InternalError(format!("json error: {e}")))?;
    if now_unix_secs() > login_state.created_at + LOGIN_STATE_TTL_SECS {
        return Err(AppError::BadRequest("state expired".into()));
    }

    Ok(login_state)
}

/// 从 Cookie 中提取值
fn extract_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;
    for segment in cookies.split(';') {
        let (k, v) = segment.trim().split_once('=')?;
        if k == name {
            return Some(v.to_string());
        }
    }
    None
}

/// 构建 Session Cookie
fn build_session_cookie(session_id: &str, web: &WebConfig) -> Result<HeaderValue, AppError> {
    let attrs = session_cookie_attrs(web);
    let cookie = format!(
        "{}={}; Path=/; Max-Age={}; HttpOnly{}{}",
        SESSION_COOKIE, session_id, SESSION_TTL_SECS, attrs.same_site, attrs.secure,
    );
    HeaderValue::from_str(&cookie)
        .map_err(|e| AppError::InternalError(format!("cookie error: {e}")))
}

fn build_clear_session_cookie(web: &WebConfig) -> Result<HeaderValue, AppError> {
    let attrs = session_cookie_attrs(web);
    let cookie = format!(
        "{}=; Path=/; Max-Age=0; HttpOnly{}{}",
        SESSION_COOKIE, attrs.same_site, attrs.secure,
    );
    HeaderValue::from_str(&cookie)
        .map_err(|e| AppError::InternalError(format!("cookie error: {e}")))
}

struct SessionCookieAttrs {
    same_site: &'static str,
    secure: &'static str,
}

fn session_cookie_attrs(web: &WebConfig) -> SessionCookieAttrs {
    let secure = web.redirect_uri.starts_with("https://")
        || web
            .frontend_origin
            .as_ref()
            .is_some_and(|o| o.starts_with("https://"));

    let same_site = if web.frontend_origin.is_some() && secure {
        "; SameSite=None"
    } else {
        if web.frontend_origin.is_some() && !secure {
            tracing::warn!(
                "frontend_origin is configured without HTTPS; using SameSite=Lax because SameSite=None requires Secure"
            );
        }
        "; SameSite=Lax"
    };

    SessionCookieAttrs {
        same_site,
        secure: if secure { "; Secure" } else { "" },
    }
}

/// 获取当前 Unix 时间戳
fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// 类型
/// OIDC 发现文档
#[derive(Deserialize, Clone)]
struct OidcDiscovery {
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: Option<String>,
}

/// OAuth 回调参数
#[derive(Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

/// OAuth 登录状态
#[derive(Serialize, Deserialize)]
struct OAuthLoginState {
    pkce_verifier: String,
    created_at: u64,
}

/// OAuth Session
#[derive(Serialize, Deserialize, Clone)]
pub struct OAuthSession {
    /// 用户信息
    pub userinfo: Value,
    /// 登录 ASN
    pub asn: u32,
    /// 是否为管理员
    pub is_admin: bool,
    /// 签发时间
    pub issued_at: u64,
    /// 过期时间
    pub expires_at: u64,
}

/// 用户信息响应
#[derive(Serialize)]
pub struct MeResponse {
    pub issued_at: u64,
    pub expires_at: u64,
    pub userinfo: Value,
}
