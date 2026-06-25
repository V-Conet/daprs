//! Agent HTTP 调用层
//!
//! 封装 server 对 agent 的所有 HTTP 请求与错误脱敏，
//! 从 api/handler.rs 抽出，供 handler/admin 共用，与 DB 逻辑解耦。

use reqwest::Method;

use shared::{AppError, PeeringPayload};

use crate::api::{FrontendAgentConfig, NodeAgentConfig};
use crate::config::AppState;

/// 向 Agent 派发请求
pub async fn dispatch_to_agent(
    state: &AppState,
    node: &str,
    method: Method,
    path: &str,
    body: Option<&PeeringPayload>,
    asn: u32,
) -> Result<(), AppError> {
    let server = state
        .config
        .server
        .servers
        .iter()
        .find(|s| s.name == node)
        .ok_or(AppError::BadRequest("node not found".into()))?;

    let url = normalize_agent_base_url(&server.address) + path;

    let mut req = state
        .http
        .clone()
        .request(method, url)
        .header("x-api-token", &state.config.server.api_token)
        .header("asn", asn.to_string());

    if let Some(b) = body {
        req = req.json(b);
    }

    let resp = req.send().await.map_err(|e| {
        tracing::error!("agent request failed: {e}");
        AppError::BadGateway
    })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::error!("agent returned {status}: {body}");
        let msg = sanitize_agent_error(&body);
        return Err(AppError::BadRequest(format!("agent error: {msg}")));
    }
    Ok(())
}

/// 获取 Agent 配置
pub async fn fetch_agent_config(
    client: &reqwest::Client,
    api_token: &str,
    address: &str,
) -> (FrontendAgentConfig, bool, Option<String>) {
    let url = format!("{}/config", normalize_agent_base_url(address));
    let resp = client
        .get(&url)
        .header("x-api-token", api_token)
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => match r.json::<FrontendAgentConfig>().await {
            Ok(conf) => (conf, true, None),
            Err(_) => (
                FrontendAgentConfig::default(),
                false,
                Some("agent returned invalid config".to_string()),
            ),
        },
        Ok(r) => (
            FrontendAgentConfig::default(),
            false,
            Some(format!("agent returned status {}", r.status())),
        ),
        Err(_) => (
            FrontendAgentConfig::default(),
            false,
            Some("agent unavailable".to_string()),
        ),
    }
}

/// 拉取单个节点配置并组装响应
pub async fn fetch_node_agent_config(
    client: &reqwest::Client,
    api_token: &str,
    server: &shared::AgentNode,
) -> NodeAgentConfig {
    let (conf, online, error) = fetch_agent_config(client, api_token, &server.address).await;
    NodeAgentConfig {
        address: server.address.clone(),
        online,
        error,
        conf,
    }
}

/// 代理转发到 Agent（cmd / peer_info 等）
pub async fn proxy_to_agent(
    state: &AppState,
    node: &str,
    method: Method,
    path: &str,
    asn: Option<u32>,
    body: Option<&serde_json::Value>,
) -> Result<serde_json::Value, AppError> {
    let server = state
        .config
        .server
        .servers
        .iter()
        .find(|s| s.name == node)
        .ok_or(AppError::BadRequest("node not found".into()))?;

    let url = normalize_agent_base_url(&server.address) + path;

    let mut req = state
        .http
        .clone()
        .request(method, url)
        .header("x-api-token", &state.config.server.api_token);
    if let Some(a) = asn {
        req = req.header("asn", a.to_string());
    }
    if let Some(b) = body {
        req = req.json(b);
    }

    let resp = req.send().await.map_err(|e| {
        tracing::error!("agent request failed: {e}");
        AppError::BadGateway
    })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::error!("agent returned {status}: {body}");
        let msg = sanitize_agent_error(&body);
        return Err(AppError::BadRequest(format!("agent error: {msg}")));
    }

    resp.json::<serde_json::Value>().await.map_err(|e| {
        tracing::error!("failed to parse agent response: {e}");
        AppError::BadGateway
    })
}

/// 代理转发到 Agent，返回原始文本（如 cmd 输出）
pub async fn proxy_to_agent_text(
    state: &AppState,
    node: &str,
    method: Method,
    path: &str,
    asn: Option<u32>,
    body: Option<&serde_json::Value>,
) -> Result<String, AppError> {
    let server = state
        .config
        .server
        .servers
        .iter()
        .find(|s| s.name == node)
        .ok_or(AppError::BadRequest("node not found".into()))?;

    let url = normalize_agent_base_url(&server.address) + path;

    let mut req = state
        .http
        .clone()
        .request(method, url)
        .header("x-api-token", &state.config.server.api_token);
    if let Some(a) = asn {
        req = req.header("asn", a.to_string());
    }
    if let Some(b) = body {
        req = req.json(b);
    }

    let resp = req.send().await.map_err(|e| {
        tracing::error!("agent request failed: {e}");
        AppError::BadGateway
    })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::error!("agent returned {status}: {body}");
        let msg = sanitize_agent_error(&body);
        return Err(AppError::BadRequest(format!("agent error: {msg}")));
    }

    resp.text().await.map_err(|e| {
        tracing::error!("failed to read agent response: {e}");
        AppError::BadGateway
    })
}

/// 标准化 Agent URL
pub fn normalize_agent_base_url(addr: &str) -> String {
    let addr = addr.trim_end_matches('/');
    if addr.starts_with("http://") || addr.starts_with("https://") {
        addr.to_string()
    } else {
        format!("http://{addr}")
    }
}

/// 移除 Agent 错误信息中的敏感信息
pub fn sanitize_agent_error(error: &str) -> String {
    let error = error.trim();
    if error.is_empty() {
        return "unknown error".into();
    }

    // 尝试从 JSON 提取错误消息
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(error)
        && let Some(msg) = json.get("error").and_then(|v| v.as_str())
    {
        return sanitize_sensitive_info(msg);
    }

    sanitize_sensitive_info(error)
}

/// 移除敏感信息
fn sanitize_sensitive_info(text: &str) -> String {
    let mut result = text.to_string();

    // 隐藏公网 IPv4 (非 DN42 范围)
    let ipv4_re = regex::Regex::new(r"\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b").unwrap();
    result = ipv4_re
        .replace_all(&result, |caps: &regex::Captures| {
            let ip = &caps[0];
            // DN42 范围: 172.20.0.0/14, 10.0.0.0/8 (部分)
            if ip.starts_with("172.2") || ip.starts_with("10.") {
                ip.to_string()
            } else {
                "[REDACTED]".to_string()
            }
        })
        .to_string();

    // 隐藏公网 IPv6 (非 DN42/ULA 范围)
    let ipv6_re = regex::Regex::new(r"\b(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}\b").unwrap();
    result = ipv6_re
        .replace_all(&result, |caps: &regex::Captures| {
            let ip = &caps[0];
            // DN42 ULA: fd00::/8, Link-local: fe80::/10
            if ip.starts_with("fd") || ip.starts_with("fe80") {
                ip.to_string()
            } else {
                "[REDACTED]".to_string()
            }
        })
        .to_string();

    // 隐藏文件路径
    let path_re = regex::Regex::new(r"/[\w/.-]+").unwrap();
    result = path_re.replace_all(&result, "[PATH]").to_string();

    // 隐藏 WireGuard 密钥 (44字符 base64)
    let key_re = regex::Regex::new(r"\b[A-Za-z0-9+/]{42,44}=\b").unwrap();
    result = key_re.replace_all(&result, "[KEY]").to_string();

    // 限制长度
    if result.len() > 300 {
        result.truncate(300);
        result.push_str("...");
    }

    result
}
