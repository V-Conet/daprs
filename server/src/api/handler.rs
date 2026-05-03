use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    api::oauth::{get_session_asn, persist_json, require_session},
    api::*,
    config::AppState,
};
use axum::http::HeaderMap;
use axum::{Json, extract::State, http::StatusCode};

const PEERING_QUEUE: &str = "peering_queue";
const MODIFY_QUEUE: &str = "modify_queue";
const REMOVE_QUEUE: &str = "remove_queue";

pub async fn get_nodes(
    State(state): State<AppState>,
) -> Result<Json<BTreeMap<String, NodeAgentConfig>>, StatusCode> {
    let client = reqwest::Client::new();
    let mut nodes = BTreeMap::new();

    for server in &state.config.server.servers {
        let (conf, online, error) =
            fetch_agent_config(&client, &state.config.server.api_token, &server.address).await;
        nodes.insert(
            server.name.clone(),
            NodeAgentConfig {
                address: server.address.clone(),
                online,
                error,
                conf,
            },
        );
    }

    Ok(Json(nodes))
}

pub async fn post_peering(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(action): Json<NodeActionRequest<PeeringPayload>>,
) -> Result<Json<bool>, StatusCode> {
    let session = require_session(&state, &headers)?;
    let asn = get_session_asn(&session).ok_or(StatusCode::UNAUTHORIZED)?;

    let key = now_nanos_key();
    persist_json(&state.db, PEERING_QUEUE, &key, &action)?;

    dispatch_to_agent(
        &state,
        &action.node,
        reqwest::Method::POST,
        "/create_peer",
        Some(&action.payload),
        asn,
    )
    .await?;

    Ok(Json(true))
}

pub async fn post_modify(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(action): Json<NodeActionRequest<PeeringPayload>>,
) -> Result<Json<bool>, StatusCode> {
    let session = require_session(&state, &headers)?;
    let asn = get_session_asn(&session).ok_or(StatusCode::UNAUTHORIZED)?;

    let key = now_nanos_key();
    persist_json(&state.db, MODIFY_QUEUE, &key, &action)?;

    dispatch_to_agent(
        &state,
        &action.node,
        reqwest::Method::POST,
        "/modify_peer",
        Some(&action.payload),
        asn,
    )
    .await?;

    Ok(Json(true))
}

pub async fn post_remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RemoveRequest>,
) -> Result<Json<bool>, StatusCode> {
    let session = require_session(&state, &headers)?;
    let asn = get_session_asn(&session).ok_or(StatusCode::UNAUTHORIZED)?;

    let key = now_nanos_key();
    persist_json(&state.db, REMOVE_QUEUE, &key, &req)?;

    dispatch_to_agent(
        &state,
        &req.node,
        reqwest::Method::DELETE,
        "/delete_peer",
        None,
        asn,
    )
    .await?;

    Ok(Json(true))
}

pub async fn get_peers(
    State(state): State<AppState>,
) -> Result<Json<Vec<NodeActionRequest<PeeringPayload>>>, StatusCode> {
    let tree = state
        .db
        .open_tree(PEERING_QUEUE)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let peers = tree
        .iter()
        .map(|item| {
            item.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
                .and_then(|(_, value)| {
                    serde_json::from_slice::<NodeActionRequest<PeeringPayload>>(&value)
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(peers))
}

// --- private ---

async fn dispatch_to_agent(
    state: &AppState,
    node: &str,
    method: reqwest::Method,
    path: &str,
    body: Option<&PeeringPayload>,
    asn: u32,
) -> Result<(), StatusCode> {
    let server = state
        .config
        .server
        .servers
        .iter()
        .find(|s| s.name == node)
        .ok_or(StatusCode::BAD_REQUEST)?;

    let url = normalize_agent_base_url(&server.address) + path;

    let mut req = reqwest::Client::new()
        .request(method, url)
        .header("x-api-token", &state.config.server.api_token)
        .header("asn", asn.to_string());

    if let Some(b) = body {
        req = req.json(b);
    }

    let resp = req.send().await.map_err(|_| StatusCode::BAD_GATEWAY)?;
    if !resp.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }
    Ok(())
}

async fn fetch_agent_config(
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
            Err(e) => (
                FrontendAgentConfig::default(),
                false,
                Some(format!("invalid config: {e}")),
            ),
        },
        Ok(r) => (
            FrontendAgentConfig::default(),
            false,
            Some(format!("agent returned {}", r.status())),
        ),
        Err(e) => (
            FrontendAgentConfig::default(),
            false,
            Some(format!("agent unavailable: {e}")),
        ),
    }
}

fn normalize_agent_base_url(addr: &str) -> String {
    let addr = addr.trim_end_matches('/');
    if addr.starts_with("http://") || addr.starts_with("https://") {
        addr.to_string()
    } else {
        format!("http://{addr}")
    }
}

fn now_nanos_key() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_string()
}
