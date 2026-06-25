//! WireGuard & Bird 配置管理模块
//!
//! 负责 WireGuard 和 Bird 的配置生成与管理。
//!
//! 模块拆分：
//! - [`mod@wg`]：WireGuard 配置生成、文件 IO、接口启停与解析
//! - [`mod@bird`]：Bird BGP 配置生成、文件 IO、birdc 控制与解析
//! - 本模块：HTTP handler 编排与输入验证

use std::{path::Path, sync::Arc};

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};

use crate::config::Config;
use crate::utils::{parse_asn_header, run_cmd};
use shared::{
    AppError, PeerInfo, PeerInfoResponse, PeeringPayload, RawCommandOutput,
    validation::{
        validate_dn42_ipv4, validate_dn42_ipv6, validate_endpoint, validate_mtu, validate_port,
        validate_wg_key,
    },
};

mod bird;
mod wg;

// API Handlers
/// 创建 Peer 配置
///
/// 生成 WireGuard 配置文件和 Bird BGP 配置文件，并启动隧道
pub async fn create_config(
    headers: HeaderMap,
    State(cfg): State<Arc<Config>>,
    Json(peer): Json<PeeringPayload>,
) -> Result<StatusCode, AppError> {
    let asn = parse_asn_header(&headers)?;

    // 输入验证
    validate_peering_payload(&peer)?;

    // 检查是否已存在
    let wg_path = wg::wg_config_path(&cfg, asn);
    let bird_path = bird::bird_config_path(&cfg, asn);

    if Path::new(&wg_path).exists() || Path::new(&bird_path).exists() {
        return Err(AppError::BadRequest(format!(
            "peer {} already exists, use modify instead",
            asn
        )));
    }

    // 生成配置
    let wg_conf = wg::build_wg_config(asn, &peer, &cfg);
    let bird_conf = bird::build_bird_config(asn, &peer);

    // 写入配置文件
    wg::write_wg(&cfg, asn, &wg_conf)?;
    bird::write_bird(&cfg, asn, &bird_conf)?;

    // 启动 WireGuard 接口
    wg::run_wg_quick_up(&cfg, asn).await?;

    // 重载 Bird 配置
    // FIXME: birdc c not always work
    bird::run_birdc_configure(&cfg).await?;

    Ok(StatusCode::CREATED)
}

/// 修改 Peer 配置
///
/// 更新已有的 WireGuard 和 Bird 配置
pub async fn modify_config(
    headers: HeaderMap,
    State(cfg): State<Arc<Config>>,
    Json(peer): Json<PeeringPayload>,
) -> Result<StatusCode, AppError> {
    let asn = parse_asn_header(&headers)?;

    // 输入验证
    validate_peering_payload(&peer)?;

    let wg_conf = wg::build_wg_config(asn, &peer, &cfg);
    let bird_conf = bird::build_bird_config(asn, &peer);

    wg::write_wg(&cfg, asn, &wg_conf)?;
    bird::write_bird(&cfg, asn, &bird_conf)?;

    // 重启 WireGuard 接口
    wg::run_wg_quick_down(&cfg, asn).await?;
    wg::run_wg_quick_up(&cfg, asn).await?;

    // 重载 Bird 配置
    bird::run_birdc_configure(&cfg).await?;

    Ok(StatusCode::OK)
}

/// 删除 Peer 配置
///
/// 删除 WireGuard 和 Bird 配置文件，并关闭隧道
pub async fn delete_config(
    headers: HeaderMap,
    State(cfg): State<Arc<Config>>,
) -> Result<StatusCode, AppError> {
    let asn = parse_asn_header(&headers)?;

    let wg_path = wg::wg_config_path(&cfg, asn);
    let bird_path = bird::bird_config_path(&cfg, asn);

    let wg_existed = Path::new(&wg_path).exists();
    let bird_existed = Path::new(&bird_path).exists();

    // 关闭 WireGuard 接口
    if wg_existed {
        wg::run_wg_quick_down(&cfg, asn).await?;
    }

    // 删除配置文件
    wg::remove_wg(&cfg, asn);
    bird::remove_bird(&cfg, asn);

    // 重载 Bird 配置
    if wg_existed || bird_existed {
        bird::run_birdc_configure(&cfg).await?;
    }

    Ok(StatusCode::OK)
}

/// 列出所有 Peer
///
/// 扫描配置目录，返回所有已配置的 Peer ASN 列表
pub async fn list_all_peers(State(cfg): State<Arc<Config>>) -> Result<Json<Vec<u32>>, AppError> {
    let mut peers = std::collections::HashSet::new();

    // 扫描 WireGuard 配置目录
    if let Ok(entries) = std::fs::read_dir(&cfg.agent.wg_path) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str()
                && let Some(asn_str) = name
                    .strip_prefix("dn42-")
                    .and_then(|s| s.strip_suffix(".conf"))
                && let Ok(asn) = asn_str.parse::<u32>()
            {
                peers.insert(asn);
            }
        }
    }

    // 扫描 Bird 配置目录
    if let Ok(entries) = std::fs::read_dir(&cfg.agent.bird_path) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str()
                && let Some(asn_str) = name.strip_suffix(".conf")
                && let Ok(asn) = asn_str.parse::<u32>()
            {
                peers.insert(asn);
            }
        }
    }

    let mut peers: Vec<u32> = peers.into_iter().collect();
    peers.sort();
    Ok(Json(peers))
}

/// 查询 Peer 信息
///
/// 返回 WireGuard 接口状态、Bird 协议状态和配置信息
pub async fn get_peer_info(
    headers: HeaderMap,
    State(cfg): State<Arc<Config>>,
) -> Result<Json<PeerInfoResponse>, AppError> {
    let asn = parse_asn_header(&headers)?;

    let wg_interface = format!("dn42-{asn}");
    let wg_path = wg::wg_config_path(&cfg, asn);
    let bird_path = bird::bird_config_path(&cfg, asn);

    // 尝试解析配置文件
    let wg_config = wg::parse_wg_config(&wg_path).ok();
    let bird_config = bird::parse_bird_config(&bird_path).ok();

    // 检查接口是否在线
    let wg_show_result = run_cmd(&cfg, "wg", &["show", &wg_interface]).await;
    let interface_up = wg_show_result.success;

    // 仅当接口在线时才有 wg_show 输出
    let wg_show = if interface_up && wg_config.is_some() {
        Some(RawCommandOutput {
            command: format!("wg show {wg_interface}"),
            output: wg_show_result.text,
        })
    } else {
        None
    };

    let bird_protocols = if bird_config.is_some() {
        let active_protocols = bird::get_active_bird_protocols(asn, &bird_path);
        let mut protocols = Vec::new();
        for protocol in active_protocols {
            protocols.push(RawCommandOutput {
                command: format!("birdc show protocols all {protocol}"),
                output: run_cmd(&cfg, "birdc", &["show", "protocols", "all", &protocol])
                    .await
                    .text,
            });
        }
        protocols
    } else {
        vec![]
    };

    // 对端信息
    let peer = wg_config.as_ref().map(|wg| PeerInfo {
        pubkey: wg.pubkey.clone(),
        endpoint: wg.endpoint.clone(),
        v4: wg.peer_v4.clone(),
        v6: wg.peer_v6.clone(),
    });

    Ok(Json(PeerInfoResponse {
        asn,
        interface_up,
        wg_show,
        bird_protocols,
        my_v4: cfg.agent.dn42.ipv4.clone(),
        my_v6: cfg.agent.dn42.ipv6.clone(),
        my_lla: cfg.agent.dn42.lla.clone(),
        my_pubkey: cfg.agent.dn42.wgkey.clone(),
        peer,
        wg: wg_config,
        bird: bird_config,
    }))
}

// 输入验证
/// 验证 Peering 请求参数
fn validate_peering_payload(peer: &PeeringPayload) -> Result<(), AppError> {
    validate_wg_key(&peer.pubkey).map_err(|e| AppError::BadRequest(e.into()))?;

    validate_endpoint(&peer.endpoint).map_err(|e| AppError::BadRequest(e.into()))?;

    if let Some(ref psk) = peer.psk {
        validate_wg_key(psk).map_err(|e| AppError::BadRequest(e.into()))?;
    }

    if let Some(ref v4) = peer.v4 {
        validate_dn42_ipv4(v4).map_err(|e| AppError::BadRequest(e.into()))?;
    }

    if let Some(ref v6) = peer.v6 {
        validate_dn42_ipv6(v6).map_err(|e| AppError::BadRequest(e.into()))?;
    }

    if let Some(mtu) = peer.mtu {
        validate_mtu(mtu).map_err(|e| AppError::BadRequest(e.into()))?;
    }

    if let Some(port) = peer.custom_port {
        validate_port(port).map_err(|e| AppError::BadRequest(e.into()))?;
    }

    Ok(())
}
