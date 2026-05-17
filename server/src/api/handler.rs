//! API Handler 模块
//!
//! 处理前端 API 请求

use std::collections::BTreeMap;

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use shared::{
    AppError, NodeActionRequest, PeeringPayload, RemoveRequest, validation::validate_asn,
};

use crate::{
    api::oauth::{get_session_asn, is_session_admin, persist_json, require_admin, require_session},
    api::{FrontendAgentConfig, NodeAgentConfig},
    config::AppState,
};

const PEERING_QUEUE: &str = "peering_queue";
const MODIFY_QUEUE: &str = "modify_queue";
const REMOVE_QUEUE: &str = "remove_queue";
const PENDING_QUEUE: &str = "pending_queue";

// 节点管理
/// 获取所有节点列表
///
/// 查询所有配置的 Agent 节点状态和配置信息
pub async fn get_nodes(
    State(state): State<AppState>,
) -> Result<Json<BTreeMap<String, NodeAgentConfig>>, AppError> {
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

// Peering 管理
/// 创建 Peering
///
/// 向指定节点发送 Peering 请求
/// 如果节点需要审核 (is_verify=true)，则放入待审核队列
pub async fn post_peering(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(action): Json<NodeActionRequest<PeeringPayload>>,
) -> Result<StatusCode, AppError> {
    let session = require_session(&state, &headers)?;
    let asn = get_session_asn(&session);

    // 获取节点配置，检查是否需要审核
    let server = state
        .config
        .server
        .servers
        .iter()
        .find(|s| s.name == action.node)
        .ok_or(AppError::BadRequest("node not found".into()))?;

    let (node_config, _online, _error) = fetch_agent_config(
        &reqwest::Client::new(),
        &state.config.server.api_token,
        &server.address,
    )
    .await;

    if node_config.is_verify {
        let request_id = uuid::Uuid::new_v4().to_string();
        let pending = PendingRequest {
            id: request_id.clone(),
            node: action.node.clone(),
            asn,
            payload: action.payload,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        persist_json(&state.db, PENDING_QUEUE, &request_id, &pending)?;
        return Ok(StatusCode::ACCEPTED);
    }

    persist_json(&state.db, PEERING_QUEUE, &action.node, &action)?;

    dispatch_to_agent(
        &state,
        &action.node,
        reqwest::Method::POST,
        "/create_peer",
        Some(&action.payload),
        asn,
    )
    .await?;

    Ok(StatusCode::CREATED)
}

/// 修改 Peering
///
/// 更新已有的 Peering 配置
pub async fn post_modify(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(action): Json<NodeActionRequest<PeeringPayload>>,
) -> Result<axum::http::StatusCode, AppError> {
    let session = require_session(&state, &headers)?;
    let asn = get_session_asn(&session);

    persist_json(&state.db, MODIFY_QUEUE, &action.node, &action)?;

    dispatch_to_agent(
        &state,
        &action.node,
        reqwest::Method::POST,
        "/modify_peer",
        Some(&action.payload),
        asn,
    )
    .await?;

    Ok(StatusCode::OK)
}

/// 删除 Peering
///
/// 删除指定的 Peering 配置
pub async fn post_remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RemoveRequest>,
) -> Result<StatusCode, AppError> {
    let session = require_session(&state, &headers)?;
    let asn = get_session_asn(&session);

    persist_json(&state.db, REMOVE_QUEUE, &req.node, &req)?;

    if let Err(e) = dispatch_to_agent(
        &state,
        &req.node,
        reqwest::Method::DELETE,
        "/delete_peer",
        None,
        asn,
    )
    .await
    {
        eprintln!(
            "dispatch_to_agent(delete_peer) failed for {}: {:?}",
            req.node, e
        );
    }

    let _ = remove_from_queue(&state.db, PEERING_QUEUE, &req.node);
    let _ = remove_from_queue(&state.db, MODIFY_QUEUE, &req.node);
    let _ = remove_from_queue(&state.db, REMOVE_QUEUE, &req.node);

    Ok(StatusCode::OK)
}

/// 获取 Peering 队列
pub async fn get_peers(
    State(state): State<AppState>,
) -> Result<Json<Vec<NodeActionRequest<PeeringPayload>>>, AppError> {
    let peers = read_all_from_queue(&state.db, PEERING_QUEUE)?;
    Ok(Json(peers))
}

/// 删除 Peering 队列项
pub async fn delete_peering_queue(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(node): Path<String>,
) -> Result<StatusCode, AppError> {
    require_session(&state, &headers)?;
    remove_from_queue(&state.db, PEERING_QUEUE, &node)?;
    Ok(StatusCode::OK)
}

/// 删除 Modify 队列项
pub async fn delete_modify_queue(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(node): Path<String>,
) -> Result<StatusCode, AppError> {
    require_session(&state, &headers)?;
    remove_from_queue(&state.db, MODIFY_QUEUE, &node)?;
    Ok(StatusCode::OK)
}

/// 删除 Remove 队列项
pub async fn delete_remove_queue(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(node): Path<String>,
) -> Result<StatusCode, AppError> {
    require_session(&state, &headers)?;
    remove_from_queue(&state.db, REMOVE_QUEUE, &node)?;
    Ok(StatusCode::OK)
}

/// 代理命令执行
///
/// 将命令请求转发到目标 Agent
pub async fn post_cmd(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(action): Json<NodeActionRequest<serde_json::Value>>,
) -> Result<String, AppError> {
    let session = require_session(&state, &headers)?;
    let _asn = get_session_asn(&session);

    let server = state
        .config
        .server
        .servers
        .iter()
        .find(|s| s.name == action.node)
        .ok_or(AppError::BadRequest("node not found".into()))?;

    let url = normalize_agent_base_url(&server.address) + "/cmd";

    let resp = reqwest::Client::new()
        .post(&url)
        .header("x-api-token", &state.config.server.api_token)
        .json(&action.payload)
        .send()
        .await
        .map_err(|_| AppError::BadGateway)?;

    if !resp.status().is_success() {
        return Err(AppError::BadGateway);
    }

    resp.text().await.map_err(|_| AppError::BadGateway)
}

/// 查询 Peer 信息
///
/// 获取指定节点上的 Peer 配置和状态
pub async fn get_peer_info(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(node): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let session = require_session(&state, &headers)?;
    let asn = get_session_asn(&session);

    let server = state
        .config
        .server
        .servers
        .iter()
        .find(|s| s.name == node)
        .ok_or(AppError::BadRequest("node not found".into()))?;

    let url = normalize_agent_base_url(&server.address) + "/peer_info";

    let resp = reqwest::Client::new()
        .get(&url)
        .header("x-api-token", &state.config.server.api_token)
        .header("asn", asn.to_string())
        .send()
        .await
        .map_err(|_| AppError::BadGateway)?;

    if !resp.status().is_success() {
        return Err(AppError::BadGateway);
    }

    let json: serde_json::Value = resp.json().await.map_err(|_| AppError::BadGateway)?;
    Ok(Json(json))
}

/// 向 Agent 发送请求
async fn dispatch_to_agent(
    state: &AppState,
    node: &str,
    method: reqwest::Method,
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

    let mut req = reqwest::Client::new()
        .request(method, url)
        .header("x-api-token", &state.config.server.api_token)
        .header("asn", asn.to_string());

    if let Some(b) = body {
        req = req.json(b);
    }

    let resp = req.send().await.map_err(|_| AppError::BadGateway)?;
    if !resp.status().is_success() {
        return Err(AppError::BadGateway);
    }
    Ok(())
}

/// 获取 Agent 配置
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

/// 标准化 Agent URL
fn normalize_agent_base_url(addr: &str) -> String {
    let addr = addr.trim_end_matches('/');
    if addr.starts_with("http://") || addr.starts_with("https://") {
        addr.to_string()
    } else {
        format!("http://{addr}")
    }
}

/// 从队列中删除
fn remove_from_queue(db: &sled::Db, tree_name: &str, node: &str) -> Result<(), AppError> {
    let tree = db
        .open_tree(tree_name)
        .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;
    tree.remove(node.as_bytes())
        .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;
    tree.flush()
        .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;
    Ok(())
}

/// 读取队列中所有项
fn read_all_from_queue<T: serde::de::DeserializeOwned>(
    db: &sled::Db,
    tree_name: &str,
) -> Result<Vec<T>, AppError> {
    let tree = db
        .open_tree(tree_name)
        .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;

    tree.iter()
        .map(|item| {
            item.map_err(|e| AppError::InternalError(format!("db error: {e}")))
                .and_then(|(_, value)| {
                    serde_json::from_slice::<T>(&value)
                        .map_err(|e| AppError::InternalError(format!("json error: {e}")))
                })
        })
        .collect::<Result<Vec<_>, _>>()
}

// 管理员 API
/// 待审核的 Peering 请求
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PendingRequest {
    /// 请求 ID
    pub id: String,
    /// 节点名称
    pub node: String,
    /// 请求者 ASN
    pub asn: u32,
    /// Peering 配置
    pub payload: PeeringPayload,
    /// 创建时间
    pub created_at: u64,
}

/// 获取待审核请求列表
pub async fn get_pending_requests(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PendingRequest>>, AppError> {
    let _session = require_admin(&state, &headers)?;
    let requests = read_all_from_queue::<PendingRequest>(&state.db, PENDING_QUEUE)?;
    Ok(Json(requests))
}

/// 批准 Peering 请求
pub async fn approve_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let _session = require_admin(&state, &headers)?;

    let pending = get_pending_request(&state.db, &request_id)?
        .ok_or(AppError::BadRequest("request not found".into()))?;

    validate_asn(pending.asn).map_err(|e| AppError::BadRequest(e.into()))?;

    dispatch_to_agent(
        &state,
        &pending.node,
        reqwest::Method::POST,
        "/create_peer",
        Some(&pending.payload),
        pending.asn,
    )
    .await?;

    remove_from_queue(&state.db, PENDING_QUEUE, &request_id)?;
    Ok(StatusCode::OK)
}

/// 拒绝 Peering 请求
pub async fn reject_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let _session = require_admin(&state, &headers)?;
    remove_from_queue(&state.db, PENDING_QUEUE, &request_id)?;
    Ok(StatusCode::OK)
}

/// 获取所有节点的 Peer 信息
pub async fn get_all_peers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<BTreeMap<String, Vec<serde_json::Value>>>, AppError> {
    let _session = require_admin(&state, &headers)?;

    let mut result = BTreeMap::new();
    let client = reqwest::Client::new();

    for server in &state.config.server.servers {
        // 获取该节点的所有 peer ASN 列表
        let peers_url = normalize_agent_base_url(&server.address) + "/peers";
        let peers_resp = client
            .get(&peers_url)
            .header("x-api-token", &state.config.server.api_token)
            .send()
            .await;

        let asns: Vec<u32> = match peers_resp {
            Ok(r) if r.status().is_success() => r.json().await.unwrap_or_default(),
            _ => {
                result.insert(
                    server.name.clone(),
                    vec![serde_json::json!({"error": "failed to fetch peer list"})],
                );
                continue;
            }
        };

        let mut peer_infos = Vec::new();
        for asn in asns {
            let info_url = normalize_agent_base_url(&server.address) + "/peer_info";
            let resp = client
                .get(&info_url)
                .header("x-api-token", &state.config.server.api_token)
                .header("asn", asn.to_string())
                .send()
                .await;

            match resp {
                Ok(r) if r.status().is_success() => {
                    if let Ok(json) = r.json::<serde_json::Value>().await {
                        peer_infos.push(json);
                    }
                }
                _ => {
                    peer_infos.push(serde_json::json!({
                        "asn": asn,
                        "error": "failed to fetch peer info"
                    }));
                }
            }
        }

        result.insert(server.name.clone(), peer_infos);
    }

    Ok(Json(result))
}

/// 管理员修改任意 Peer
pub async fn admin_modify_peer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AdminPeerRequest>,
) -> Result<StatusCode, AppError> {
    let _session = require_admin(&state, &headers)?;

    // 验证 ASN
    validate_asn(req.asn).map_err(|e| AppError::BadRequest(e.into()))?;

    dispatch_to_agent(
        &state,
        &req.node,
        reqwest::Method::POST,
        "/modify_peer",
        Some(&req.payload),
        req.asn,
    )
    .await?;

    Ok(StatusCode::OK)
}

/// 管理员删除任意 Peer
pub async fn admin_delete_peer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AdminDeleteRequest>,
) -> Result<StatusCode, AppError> {
    let _session = require_admin(&state, &headers)?;

    // 验证 ASN
    validate_asn(req.asn).map_err(|e| AppError::BadRequest(e.into()))?;

    dispatch_to_agent(
        &state,
        &req.node,
        reqwest::Method::DELETE,
        "/delete_peer",
        None,
        req.asn,
    )
    .await?;

    Ok(StatusCode::OK)
}

/// 管理员 Peer 修改请求
#[derive(Deserialize)]
pub struct AdminPeerRequest {
    pub node: String,
    pub asn: u32,
    pub payload: PeeringPayload,
}

/// 管理员删除请求
#[derive(Deserialize)]
pub struct AdminDeleteRequest {
    pub node: String,
    pub asn: u32,
}

/// 获取单个待审核请求
fn get_pending_request(db: &sled::Db, id: &str) -> Result<Option<PendingRequest>, AppError> {
    let tree = db
        .open_tree(PENDING_QUEUE)
        .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;

    match tree.get(id.as_bytes()) {
        Ok(Some(value)) => {
            let req: PendingRequest = serde_json::from_slice(&value)
                .map_err(|e| AppError::InternalError(format!("json error: {e}")))?;
            Ok(Some(req))
        }
        _ => Ok(None),
    }
}

/// 检查是否为管理员
pub async fn check_admin(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<bool>, AppError> {
    let session = require_session(&state, &headers)?;
    Ok(Json(is_session_admin(&session)))
}
