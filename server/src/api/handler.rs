//! API Handler 模块
//!
//! 处理前端 API 请求
//! 鉴权 → 仓储层 → agent_client → 审计

use std::collections::BTreeMap;

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use reqwest::Method;
use shared::{
    ActionResult, ActionType, AppError, NodeActionRequest, PeeringPayload, RemoveRequest,
    validation::validate_asn,
};

use crate::{
    agent_client,
    api::{
        AdminDeleteRequest, AdminPeerRequest, NodeAgentConfig, PendingRequest,
        oauth::{get_session_asn, is_session_admin, require_admin, require_session},
    },
    config::AppState,
    db::{self, peer_cache, peering_requests},
};

const PENDING_TTL_SECS: i64 = 7 * 24 * 60 * 60; // 7 days
const AUDIT_DEFAULT_LIMIT: i64 = 1000;
const AUDIT_DEFAULT_OFFSET: i64 = 0;

/// 记录审计日志（失败仅记日志，不影响主流程）
async fn record_audit(
    state: &AppState,
    actor_asn: u32,
    action: ActionType,
    target_asn: u32,
    node: &str,
    result: ActionResult,
) {
    let id = uuid::Uuid::new_v4().to_string();
    if let Err(e) = db::audit::insert(
        &state.pool,
        &id,
        actor_asn as i64,
        action,
        target_asn as i64,
        node,
        &result,
    )
    .await
    {
        tracing::error!("failed to write audit log: {e}");
    }
}

// 节点管理
/// 获取所有节点列表
///
/// 查询所有配置的 Agent 节点状态和配置信息
pub async fn get_nodes(
    State(state): State<AppState>,
) -> Result<Json<BTreeMap<String, NodeAgentConfig>>, AppError> {
    let client = state.http.clone();
    let mut nodes = BTreeMap::new();

    for server in &state.config.server.servers {
        let conf =
            agent_client::fetch_node_agent_config(&client, &state.config.server.api_token, server)
                .await;
        nodes.insert(server.name.clone(), conf);
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
    let ctx = require_session(&state, &headers).await?;
    let asn = get_session_asn(&ctx);

    // 获取节点配置，检查是否需要审核
    let server = state
        .config
        .server
        .servers
        .iter()
        .find(|s| s.name == action.node)
        .ok_or(AppError::BadRequest("node not found".into()))?;

    let (node_config, _online, _error) = agent_client::fetch_agent_config(
        &state.http,
        &state.config.server.api_token,
        &server.address,
    )
    .await;

    let require_approval = node_config.is_verify;
    let request_id = uuid::Uuid::new_v4().to_string();
    let payload_json = serde_json::to_string(&action.payload)
        .map_err(|e| AppError::InternalError(format!("payload encode: {e}")))?;
    let expires_at = if require_approval {
        Some(db::now_unix_secs() + PENDING_TTL_SECS)
    } else {
        None
    };

    peering_requests::create(
        &state.pool,
        &request_id,
        asn as i64,
        &action.node,
        peering_requests::Action::Create,
        require_approval,
        Some(&payload_json),
        expires_at,
    )
    .await?;

    if require_approval {
        record_audit(
            &state,
            asn,
            ActionType::Create,
            asn,
            &action.node,
            ActionResult::Success,
        )
        .await;
        return Ok(StatusCode::ACCEPTED);
    }

    let result = agent_client::dispatch_to_agent(
        &state,
        &action.node,
        Method::POST,
        "/create_peer",
        Some(&action.payload),
        asn,
    )
    .await;

    match &result {
        Ok(()) => {
            peering_requests::mark_dispatched(&state.pool, &request_id).await?;
            peering_requests::mark_succeeded(&state.pool, &request_id).await?;
            peer_cache::upsert_active(&state.pool, asn as i64, &action.node, &payload_json).await?;
            record_audit(
                &state,
                asn,
                ActionType::Create,
                asn,
                &action.node,
                ActionResult::Success,
            )
            .await;
        }
        Err(e) => {
            peering_requests::mark_failed(&state.pool, &request_id, &e.to_string()).await?;
            record_audit(
                &state,
                asn,
                ActionType::Create,
                asn,
                &action.node,
                ActionResult::Failed(e.to_string()),
            )
            .await;
        }
    }

    result?;
    Ok(StatusCode::CREATED)
}

/// 修改 Peering
///
/// 更新已有的 Peering 配置
pub async fn post_modify(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(action): Json<NodeActionRequest<PeeringPayload>>,
) -> Result<StatusCode, AppError> {
    let ctx = require_session(&state, &headers).await?;
    let asn = get_session_asn(&ctx);

    let request_id = uuid::Uuid::new_v4().to_string();
    let payload_json = serde_json::to_string(&action.payload)
        .map_err(|e| AppError::InternalError(format!("payload encode: {e}")))?;

    peering_requests::create(
        &state.pool,
        &request_id,
        asn as i64,
        &action.node,
        peering_requests::Action::Modify,
        false,
        Some(&payload_json),
        None,
    )
    .await?;

    let result = agent_client::dispatch_to_agent(
        &state,
        &action.node,
        Method::POST,
        "/modify_peer",
        Some(&action.payload),
        asn,
    )
    .await;

    match &result {
        Ok(()) => {
            peering_requests::mark_dispatched(&state.pool, &request_id).await?;
            peering_requests::mark_succeeded(&state.pool, &request_id).await?;
            peer_cache::update_payload(&state.pool, asn as i64, &action.node, &payload_json)
                .await?;
            record_audit(
                &state,
                asn,
                ActionType::Modify,
                asn,
                &action.node,
                ActionResult::Success,
            )
            .await;
        }
        Err(e) => {
            peering_requests::mark_failed(&state.pool, &request_id, &e.to_string()).await?;
            record_audit(
                &state,
                asn,
                ActionType::Modify,
                asn,
                &action.node,
                ActionResult::Failed(e.to_string()),
            )
            .await;
        }
    }

    result?;
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
    let ctx = require_session(&state, &headers).await?;
    let asn = get_session_asn(&ctx);

    let request_id = uuid::Uuid::new_v4().to_string();
    peering_requests::create(
        &state.pool,
        &request_id,
        asn as i64,
        &req.node,
        peering_requests::Action::Delete,
        false,
        None,
        None,
    )
    .await?;

    let result = agent_client::dispatch_to_agent(
        &state,
        &req.node,
        Method::DELETE,
        "/delete_peer",
        None,
        asn,
    )
    .await;

    match &result {
        Ok(()) => {
            peering_requests::mark_dispatched(&state.pool, &request_id).await?;
            peering_requests::mark_succeeded(&state.pool, &request_id).await?;
            peer_cache::mark_removed(&state.pool, asn as i64, &req.node).await?;
            record_audit(
                &state,
                asn,
                ActionType::Delete,
                asn,
                &req.node,
                ActionResult::Success,
            )
            .await;
        }
        Err(e) => {
            peering_requests::mark_failed(&state.pool, &request_id, &e.to_string()).await?;
            record_audit(
                &state,
                asn,
                ActionType::Delete,
                asn,
                &req.node,
                ActionResult::Failed(e.to_string()),
            )
            .await;
        }
    }

    result?;
    Ok(StatusCode::OK)
}

/// 获取当前用户的 Peering 列表（来自 peer_cache，非权威）
pub async fn get_peers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<NodeActionRequest<PeeringPayload>>>, AppError> {
    let ctx = require_session(&state, &headers).await?;
    let asn = get_session_asn(&ctx);

    let rows = peer_cache::list_active_for_user(&state.pool, asn as i64).await?;
    let peers = rows
        .into_iter()
        .filter_map(|r| match r.payload_value() {
            Ok(payload) => Some(NodeActionRequest {
                node: r.node,
                payload,
            }),
            Err(e) => {
                tracing::error!("skip corrupt peer_cache row: {e}");
                None
            }
        })
        .collect();
    Ok(Json(peers))
}

/// 取消 Peering 队列项
pub async fn delete_peering_queue(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(node): Path<String>,
) -> Result<StatusCode, AppError> {
    let ctx = require_session(&state, &headers).await?;
    let asn = get_session_asn(&ctx);
    peering_requests::cancel_for_user_node(&state.pool, asn as i64, &node).await?;
    Ok(StatusCode::OK)
}

/// 取消 Modify 队列项
pub async fn delete_modify_queue(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(node): Path<String>,
) -> Result<StatusCode, AppError> {
    let ctx = require_session(&state, &headers).await?;
    let asn = get_session_asn(&ctx);
    peering_requests::cancel_for_user_node(&state.pool, asn as i64, &node).await?;
    Ok(StatusCode::OK)
}

/// 取消 Remove 队列项
pub async fn delete_remove_queue(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(node): Path<String>,
) -> Result<StatusCode, AppError> {
    let ctx = require_session(&state, &headers).await?;
    let asn = get_session_asn(&ctx);
    peering_requests::cancel_for_user_node(&state.pool, asn as i64, &node).await?;
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
    let _ctx = require_session(&state, &headers).await?;

    agent_client::proxy_to_agent_text(
        &state,
        &action.node,
        Method::POST,
        "/cmd",
        None,
        Some(&action.payload),
    )
    .await
}

/// 查询 Peer 信息
///
/// 获取指定节点上的 Peer 配置和状态（回源 agent，权威）
pub async fn get_peer_info(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(node): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ctx = require_session(&state, &headers).await?;
    let asn = get_session_asn(&ctx);

    let json =
        agent_client::proxy_to_agent(&state, &node, Method::GET, "/peer_info", Some(asn), None)
            .await?;
    Ok(Json(json))
}

// 管理员 API
/// 获取用户的待处理请求
pub async fn get_my_pending_requests(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PendingRequest>>, AppError> {
    let ctx = require_session(&state, &headers).await?;
    let asn = get_session_asn(&ctx);

    peering_requests::cleanup_expired(&state.pool).await?;
    let rows = peering_requests::list_pending_for_user(&state.pool, asn as i64).await?;
    Ok(Json(rows.into_iter().filter_map(req_to_pending).collect()))
}

/// 获取待审核请求列表
pub async fn get_pending_requests(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PendingRequest>>, AppError> {
    require_admin(&state, &headers).await?;
    peering_requests::cleanup_expired(&state.pool).await?;
    let rows = peering_requests::list_all_pending(&state.pool).await?;
    Ok(Json(rows.into_iter().filter_map(req_to_pending).collect()))
}

/// 批准 Peering 请求
pub async fn approve_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let ctx = require_admin(&state, &headers).await?;
    let actor_asn = get_session_asn(&ctx);

    let pending = peering_requests::get(&state.pool, &request_id)
        .await?
        .ok_or(AppError::BadRequest("request not found".into()))?;

    let target_asn = pending.user_asn as u32;
    let payload = pending
        .payload_value()?
        .ok_or(AppError::BadRequest("missing payload".into()))?;
    let node = pending.node.clone();

    validate_asn(target_asn).map_err(|e| AppError::BadRequest(e.into()))?;

    let result = agent_client::dispatch_to_agent(
        &state,
        &node,
        Method::POST,
        "/create_peer",
        Some(&payload),
        target_asn,
    )
    .await;

    match &result {
        Ok(()) => {
            peering_requests::mark_approved(&state.pool, &request_id, actor_asn as i64).await?;
            peering_requests::mark_dispatched(&state.pool, &request_id).await?;
            peering_requests::mark_succeeded(&state.pool, &request_id).await?;
            let payload_json = serde_json::to_string(&payload)
                .map_err(|e| AppError::InternalError(format!("payload encode: {e}")))?;
            peer_cache::upsert_active(&state.pool, target_asn as i64, &node, &payload_json).await?;
            record_audit(
                &state,
                actor_asn,
                ActionType::Approve,
                target_asn,
                &node,
                ActionResult::Success,
            )
            .await;
        }
        Err(e) => {
            record_audit(
                &state,
                actor_asn,
                ActionType::Approve,
                target_asn,
                &node,
                ActionResult::Failed(e.to_string()),
            )
            .await;
        }
    }

    result?;
    Ok(StatusCode::OK)
}

/// 拒绝 Peering 请求
pub async fn reject_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let ctx = require_admin(&state, &headers).await?;
    let actor_asn = get_session_asn(&ctx);

    let pending = peering_requests::get(&state.pool, &request_id)
        .await?
        .ok_or(AppError::BadRequest("request not found".into()))?;

    peering_requests::mark_rejected(&state.pool, &request_id, actor_asn as i64).await?;

    record_audit(
        &state,
        actor_asn,
        ActionType::Reject,
        pending.user_asn as u32,
        &pending.node,
        ActionResult::Success,
    )
    .await;

    Ok(StatusCode::OK)
}

/// 获取所有节点的 Peer 信息（回源 agent，权威）
pub async fn get_all_peers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<BTreeMap<String, Vec<serde_json::Value>>>, AppError> {
    require_admin(&state, &headers).await?;

    let mut result = BTreeMap::new();
    let client = state.http.clone();

    for server in &state.config.server.servers {
        let peers_url = agent_client::normalize_agent_base_url(&server.address) + "/peers";
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
            let info_url = agent_client::normalize_agent_base_url(&server.address) + "/peer_info";
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
    let ctx = require_admin(&state, &headers).await?;
    let actor_asn = get_session_asn(&ctx);

    validate_asn(req.asn).map_err(|e| AppError::BadRequest(e.into()))?;

    let payload_json = serde_json::to_string(&req.payload)
        .map_err(|e| AppError::InternalError(format!("payload encode: {e}")))?;
    let request_id = uuid::Uuid::new_v4().to_string();

    let result = agent_client::dispatch_to_agent(
        &state,
        &req.node,
        Method::POST,
        "/modify_peer",
        Some(&req.payload),
        req.asn,
    )
    .await;

    let status = if result.is_ok() {
        peering_requests::create_terminal(
            &state.pool,
            &request_id,
            req.asn as i64,
            &req.node,
            peering_requests::Action::Modify,
            Some(&payload_json),
            peering_requests::Status::Succeeded,
            Some(actor_asn as i64),
            None,
        )
        .await?;
        peer_cache::update_payload(&state.pool, req.asn as i64, &req.node, &payload_json).await?;
        record_audit(
            &state,
            actor_asn,
            ActionType::Modify,
            req.asn,
            &req.node,
            ActionResult::Success,
        )
        .await;
        StatusCode::OK
    } else {
        let err = result.as_ref().err().unwrap().to_string();
        peering_requests::create_terminal(
            &state.pool,
            &request_id,
            req.asn as i64,
            &req.node,
            peering_requests::Action::Modify,
            Some(&payload_json),
            peering_requests::Status::Failed,
            Some(actor_asn as i64),
            Some(&err),
        )
        .await?;
        record_audit(
            &state,
            actor_asn,
            ActionType::Modify,
            req.asn,
            &req.node,
            ActionResult::Failed(err),
        )
        .await;
        StatusCode::OK
    };

    result?;
    Ok(status)
}

/// 管理员删除任意 Peer
pub async fn admin_delete_peer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AdminDeleteRequest>,
) -> Result<StatusCode, AppError> {
    let ctx = require_admin(&state, &headers).await?;
    let actor_asn = get_session_asn(&ctx);

    validate_asn(req.asn).map_err(|e| AppError::BadRequest(e.into()))?;

    let request_id = uuid::Uuid::new_v4().to_string();

    let result = agent_client::dispatch_to_agent(
        &state,
        &req.node,
        Method::DELETE,
        "/delete_peer",
        None,
        req.asn,
    )
    .await;

    let status = if result.is_ok() {
        peering_requests::create_terminal(
            &state.pool,
            &request_id,
            req.asn as i64,
            &req.node,
            peering_requests::Action::Delete,
            None,
            peering_requests::Status::Succeeded,
            Some(actor_asn as i64),
            None,
        )
        .await?;
        peer_cache::mark_removed(&state.pool, req.asn as i64, &req.node).await?;
        record_audit(
            &state,
            actor_asn,
            ActionType::Delete,
            req.asn,
            &req.node,
            ActionResult::Success,
        )
        .await;
        StatusCode::OK
    } else {
        let err = result.as_ref().err().unwrap().to_string();
        peering_requests::create_terminal(
            &state.pool,
            &request_id,
            req.asn as i64,
            &req.node,
            peering_requests::Action::Delete,
            None,
            peering_requests::Status::Failed,
            Some(actor_asn as i64),
            Some(&err),
        )
        .await?;
        record_audit(
            &state,
            actor_asn,
            ActionType::Delete,
            req.asn,
            &req.node,
            ActionResult::Failed(err),
        )
        .await;
        StatusCode::OK
    };

    result?;
    Ok(status)
}

/// 检查是否为管理员
pub async fn check_admin(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<bool>, AppError> {
    let ctx = require_session(&state, &headers).await?;
    Ok(Json(is_session_admin(&state, &ctx)))
}

/// 获取审计日志（分页，默认最近 1000 条）
pub async fn get_audit_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<shared::AuditLog>>, AppError> {
    require_admin(&state, &headers).await?;
    let rows =
        db::audit::list_paginated(&state.pool, AUDIT_DEFAULT_LIMIT, AUDIT_DEFAULT_OFFSET).await?;
    let logs = rows
        .into_iter()
        .filter_map(|r| match r.to_audit_log() {
            Ok(log) => Some(log),
            Err(e) => {
                tracing::error!("skip corrupt audit row: {e}");
                None
            }
        })
        .collect();
    Ok(Json(logs))
}

/// 将请求行转为 API 响应类型
fn req_to_pending(r: peering_requests::PeeringRequest) -> Option<PendingRequest> {
    let payload = r.payload_value().ok()??;
    Some(PendingRequest {
        id: r.id,
        node: r.node,
        asn: r.user_asn as u32,
        payload,
        created_at: r.created_at as u64,
    })
}
