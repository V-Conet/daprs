use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;

use crate::{api::*, config::AppState};

// TODO: probably return StatusCode instead of Json<bool> for better support for webui

const PEERING_QUEUE: &str = "peering_queue";
const MODIFY_QUEUE: &str = "modify_queue";
const REMOVE_QUEUE: &str = "remove_queue";

// 获取所有节点的配置信息,包含agent的具体配置,json格式返回给webui
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

/// 处理peering请求
// accroding to agent/src/api.rs
// create/modify peer better return the full wg/bird config
// so that, agent will only need to apply the config, without generating it by itself, which is more flexible and less error-prone
// SO FAR, the wg/bird config will not be generated, which means return placeholder str,
// the reason is that, the config generation logic is not yet implemented, and the logic is quite complex
// it need to be implemented in the very end

pub async fn post_peering(
    State(state): State<AppState>,
    Json(action): Json<NodeActionRequest<PeeringPayload>>,
) -> Result<Json<bool>, StatusCode> {
    queue_json(&state.db, PEERING_QUEUE, &action)?;
    dispatch_create_peer(&state, &action.node, &action.payload).await?;
    Ok(Json(true))
}

// 类似post_peering
pub async fn post_modify(
    State(state): State<AppState>,
    Json(action): Json<NodeActionRequest<PeeringPayload>>,
) -> Result<Json<bool>, StatusCode> {
    queue_json(&state.db, MODIFY_QUEUE, &action)?;
    dispatch_modify_peer(&state, &action.node, &action.payload).await?;
    Ok(Json(true))
}

pub async fn post_remove(
    State(state): State<AppState>,
    Json(payload): Json<PeerInfo>,
) -> Result<Json<bool>, StatusCode> {
    queue_json(&state.db, REMOVE_QUEUE, &payload)?;
    dispatch_delete_peer(&state, &payload.node).await?;
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

// 将peering请求序列化为json并存储在sled数据库中,作为agent的任务队列
// which is very useful when server/agent connection is not stable
// also, this can be shown in webui for admin/user to check the pending peering requests

fn queue_json<T: Serialize>(db: &sled::Db, tree_name: &str, value: &T) -> Result<(), StatusCode> {
    let tree = db
        .open_tree(tree_name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let key = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .as_nanos()
        .to_string();
    let bytes = serde_json::to_vec(value).map_err(|_| StatusCode::BAD_REQUEST)?;

    tree.insert(key.as_bytes(), bytes)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    tree.flush()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
        Ok(r) => {
            if !r.status().is_success() {
                return (
                    default_frontend_config(),
                    false,
                    Some(format!("agent returned {}", r.status())),
                );
            }

            match r.json::<FrontendAgentConfig>().await {
                Ok(conf) => (conf, true, None),
                Err(e) => (
                    default_frontend_config(),
                    false,
                    Some(format!("invalid agent config response: {e}")),
                ),
            }
        }
        Err(e) => (
            default_frontend_config(),
            false,
            Some(format!("agent unavailable: {e}")),
        ),
    }
}

async fn dispatch_create_peer(
    state: &AppState,
    node: &str,
    payload: &PeeringPayload,
) -> Result<(), StatusCode> {
    dispatch_wb(
        state,
        node,
        "/create_peer",
        &WbConfig {
            wg_config: format!(
                "# peering\n# is_mhp={} is_nhp={} policy={:?}\n",
                payload.is_mhp, payload.is_nhp, payload.policy
            ),
            bird_config: format!("# peering\n# policy={:?}\n", payload.policy),
        },
    )
    .await
}

async fn dispatch_modify_peer(
    state: &AppState,
    node: &str,
    payload: &PeeringPayload,
) -> Result<(), StatusCode> {
    dispatch_wb(
        state,
        node,
        "/modify_peer",
        &WbConfig {
            wg_config: format!(
                "# modify\n# is_mhp={} is_nhp={} policy={:?}\n",
                payload.is_mhp, payload.is_nhp, payload.policy
            ),
            bird_config: format!("# modify\n# policy={:?}\n", payload.policy),
        },
    )
    .await
}

async fn dispatch_delete_peer(state: &AppState, node: &str) -> Result<(), StatusCode> {
    let server = state
        .config
        .server
        .servers
        .iter()
        .find(|s| s.name == node)
        .ok_or(StatusCode::BAD_REQUEST)?;
    let url = format!("{}/delete_peer", normalize_agent_base_url(&server.address));

    let client = reqwest::Client::new();
    let resp = client
        .delete(url)
        .header("x-api-token", state.config.server.api_token.clone())
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    if !resp.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }
    Ok(())
}

async fn dispatch_wb(
    state: &AppState,
    node: &str,
    path: &str,
    config: &WbConfig,
) -> Result<(), StatusCode> {
    let server = state
        .config
        .server
        .servers
        .iter()
        .find(|s| s.name == node)
        .ok_or(StatusCode::BAD_REQUEST)?;
    let url = format!("{}{}", normalize_agent_base_url(&server.address), path);

    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("x-api-token", state.config.server.api_token.clone())
        .json(config)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    if !resp.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }
    Ok(())
}

fn normalize_agent_base_url(address: &str) -> String {
    if address.starts_with("http://") || address.starts_with("https://") {
        return address.trim_end_matches('/').to_string();
    }

    format!("http://{}", address.trim_end_matches('/'))
}

fn default_frontend_config() -> FrontendAgentConfig {
    FrontendAgentConfig {
        version: 1,
        is_open: false,
        is_verify: false,
        extra_msg: "agent unavailable".to_string(),
        net: crate::config::NetConfig {
            ipv4: false,
            ipv6: false,
            accept_nat: false,
            cn: false,
        },
        dn42: crate::config::Dn42Config {
            asn: 0,
            ipv4: String::new(),
            ipv6: String::new(),
            lla: String::new(),
            wgkey: String::new(),
        },
    }
}
