// Write Wireguard & Bird configuration files

use std::process::Command;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;

use crate::config::Config;

#[derive(Deserialize)]
pub struct PeerRequest {
    /// MultiHop
    pub is_mhp: bool,
    /// Extended NextHop
    pub is_nhp: bool,
    /// 路由策略
    pub policy: RoutingPolicy,
    /// 本节点 DN42 IPv4
    pub v4: Option<String>,
    /// 本节点 DN42 IPv6
    pub v6: Option<String>,
    /// 本节点链路本地地址
    pub lla: Option<String>,
    /// 是否优先使用链路本地地址
    pub is_prefer_lla: bool,
    /// 对端地址
    pub endpoint: String,
    /// 对端公钥
    pub pubkey: String,
    /// 开放给对端的端口
    pub custom_port: Option<u16>,
    /// 预共享密钥
    pub psk: Option<String>,
    /// MTU
    pub mtu: Option<u16>,
}

#[derive(Deserialize, Debug)]
pub enum RoutingPolicy {
    /// 接收所有有效路由，导出所有有效路由。
    FullTable,
    /// 接收所有有效路由，只导出本地自有路由。
    Transit,
    /// 只接收 AS_PATH = 1 的路由，导出本地自有路由和直接下游路由。
    PeeringOnly,
    /// 只接收 AS_PATH = 1 的路由，导出所有有效路由。
    Downstream,
}

pub async fn create_config(
    headers: HeaderMap,
    State(cfg): State<Config>,
    Json(peer): Json<PeerRequest>,
) -> Result<StatusCode, StatusCode> {
    let asn = parse_asn_header(&headers)?;

    let wg_conf = build_wg_config(asn, &peer, &cfg);
    let bird_conf = build_bird_config(asn, &peer);

    write_wg(&cfg, asn, &wg_conf)?;
    write_bird(&cfg, asn, &bird_conf)?;

    // run_wg_up(asn);
    // run_birdc_configure();

    Ok(StatusCode::OK)
}

pub async fn modify_config(
    headers: HeaderMap,
    State(cfg): State<Config>,
    Json(peer): Json<PeerRequest>,
) -> Result<StatusCode, StatusCode> {
    let asn = parse_asn_header(&headers)?;

    let wg_conf = build_wg_config(asn, &peer, &cfg);
    let bird_conf = build_bird_config(asn, &peer);

    write_wg(&cfg, asn, &wg_conf)?;
    write_bird(&cfg, asn, &bird_conf)?;

    // run_wg_down(asn);
    // run_wg_up(asn);
    // run_birdc_configure();

    Ok(StatusCode::OK)
}

pub async fn delete_config(
    headers: HeaderMap,
    State(cfg): State<Config>,
) -> Result<StatusCode, StatusCode> {
    let asn = parse_asn_header(&headers)?;

    // run_wg_down(asn);
    remove_wg(&cfg, asn);
    remove_bird(&cfg, asn);
    // run_birdc_configure();

    Ok(StatusCode::OK)
}

// --- config builders ---

fn build_wg_config(asn: u32, peer: &PeerRequest, cfg: &Config) -> String {
    // Use peer-requested LLA if provided, otherwise the agent's own LLA.
    let my_lla = peer.lla.as_deref().unwrap_or(&cfg.agent.dn42.lla);
    let my_ula = &cfg.agent.dn42.ipv6;
    let my_v4 = &cfg.agent.dn42.ipv4;
    
    // default: last five digits of ASN if it's 424242XXXX, for Clearnet ASNs, using 40000 + last 4 digits
    let port = peer.custom_port.unwrap_or({
        if asn >= 4242420000 {
            (asn % 100000) as u16
        } else {
            40000 + (asn % 10000) as u16
        }
    });
    let mtu = peer.mtu.unwrap_or(1420);

    let (ll_peer, ula_peer, v4_peer) = classify_peer_ips(peer);

    let psk_line = peer
        .psk
        .as_ref()
        .map(|k| format!("PresharedKey = {k}\n"))
        .unwrap_or_default();

    format!(
        "# {asn}\n\
         [Interface]\n\
         ListenPort = {port}\n\
         Table = off\n\
         MTU = {mtu}\n\
         PostUp = wg set %i private-key /etc/wireguard/dn42-privatekey\n\
         PostUp = ip addr add {my_lla}/64{ll_peer} dev %i\n\
         PostUp = ip addr add {my_ula}/128{ula_peer} dev %i\n\
         PostUp = ip addr add {my_v4}/32{v4_peer} dev %i\n\
         [Peer]\n\
         PublicKey = {pubkey}\n\
         {psk}\
         Endpoint = {endpoint}\n\
         AllowedIPs = 172.20.0.0/14, 10.0.0.0/8, 172.31.0.0/16, fd00::/8, fe80::/64\n",
        psk = psk_line,
        pubkey = peer.pubkey,
        endpoint = peer.endpoint,
    )
}

fn classify_peer_ips(peer: &PeerRequest) -> (String, String, String) {
    let ll = peer
        .v6
        .as_ref()
        .filter(|v6| v6.starts_with("fe80:"))
        .map(|v6| format!(" peer {v6}/64"));

    let ula_peer = peer.v6.as_ref().filter(|v6| !v6.starts_with("fe80:"));

    let ula = ula_peer.map(|v6| format!(" peer {v6}/128"));

    let v4 = peer.v4.as_ref().map(|v4| format!(" peer {v4}/32"));

    (
        ll.unwrap_or_default(),
        ula.unwrap_or_default(),
        v4.unwrap_or_default(),
    )
}

fn build_bird_config(asn: u32, peer: &PeerRequest) -> String {
    let has_v4 = peer.v4.is_some();
    let has_v6 = peer.v6.is_some();

    match (has_v6, has_v4) {
        (true, true) if peer.is_mhp => {
            // Single MP-BGP session. ENH (is_nhp) implies v6 carries both AFs.
            if peer.is_nhp {
                gen_bird_protocol(
                    asn,
                    6,
                    None,
                    &peer.policy,
                    peer.is_prefer_lla,
                    peer.v6.as_deref(),
                    peer.v4.as_deref(),
                )
            } else {
                gen_bird_protocol(
                    asn,
                    4,
                    None,
                    &peer.policy,
                    peer.is_prefer_lla,
                    peer.v6.as_deref(),
                    peer.v4.as_deref(),
                )
            }
        }
        (true, true) => {
            // Two separate sessions, each blocks the other AF
            format!(
                "{}\n{}",
                gen_bird_protocol(
                    asn,
                    6,
                    Some(4),
                    &peer.policy,
                    peer.is_prefer_lla,
                    peer.v6.as_deref(),
                    None
                ),
                gen_bird_protocol(
                    asn,
                    4,
                    Some(6),
                    &peer.policy,
                    peer.is_prefer_lla,
                    None,
                    peer.v4.as_deref()
                ),
            )
        }
        (true, false) => gen_bird_protocol(
            asn,
            6,
            Some(4),
            &peer.policy,
            peer.is_prefer_lla,
            peer.v6.as_deref(),
            None,
        ),
        (false, true) => gen_bird_protocol(
            asn,
            4,
            Some(6),
            &peer.policy,
            peer.is_prefer_lla,
            None,
            peer.v4.as_deref(),
        ),
        (false, false) => String::new(),
    }
}

fn gen_bird_protocol(
    asn: u32,
    version: u8,
    block_af: Option<u8>,
    policy: &RoutingPolicy,
    is_prefer_lla: bool,
    v6: Option<&str>,
    v4: Option<&str>,
) -> String {
    let neighbor = match version {
        6 => v6.unwrap_or(""),
        4 => v4.unwrap_or(""),
        _ => "",
    };

    let filter_comment = policy_filter_comment(policy);
    let lla_note = if is_prefer_lla { " (prefer LLA)" } else { "" };

    let mut text = format!(
        "protocol bgp DN42_{asn}_v{version} from dn42_peers {{\n\
         \x20   neighbor {neighbor} % 'dn42-{asn}' external;\n\
         \x20   # policy: {policy:?}{filter_comment}{lla_note}\n"
    );

    if let Some(af) = block_af {
        text.push_str(&format!(
            "    ipv{af} {{\n\
             \x20       import none;\n\
             \x20       export none;\n\
             \x20   }};\n"
        ));
    }

    text.push_str("}\n");
    text
}

fn policy_filter_comment(policy: &RoutingPolicy) -> &str {
    match policy {
        RoutingPolicy::FullTable => "",
        RoutingPolicy::Transit => " (import all, export defined by dn42_peers)",
        RoutingPolicy::PeeringOnly => " (import defined by dn42_peers, export all)",
        RoutingPolicy::Downstream => " (import/export defined by dn42_peers)",
    }
}

// --- filesystem helpers ---

fn write_wg(cfg: &Config, asn: u32, conf: &str) -> Result<(), StatusCode> {
    let path = format!("{}/dn42-{}.conf", cfg.agent.wg_path, asn);
    std::fs::write(&path, conf).map_err(|e| {
        eprintln!("wg write error {path}: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

fn write_bird(cfg: &Config, asn: u32, conf: &str) -> Result<(), StatusCode> {
    let path = format!("{}/dn42_peers/{}.conf", cfg.agent.bird_path, asn);
    std::fs::create_dir_all(format!("{}/dn42_peers", cfg.agent.bird_path)).ok();
    std::fs::write(&path, conf).map_err(|e| {
        eprintln!("bird write error {path}: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

fn remove_wg(cfg: &Config, asn: u32) {
    let path = format!("{}/dn42-{}.conf", cfg.agent.wg_path, asn);
    if let Err(e) = std::fs::remove_file(&path) {
        eprintln!("wg remove warning {path}: {e}");
    }
}

fn remove_bird(cfg: &Config, asn: u32) {
    let path = format!("{}/dn42_peers/{}.conf", cfg.agent.bird_path, asn);
    if let Err(e) = std::fs::remove_file(&path) {
        eprintln!("bird remove warning {path}: {e}");
    }
}

// --- command runners ---

fn run_wg_up(asn: u32) {
    let out = run_cmd("wg-quick", &["up", &format!("dn42-{asn}")]);
    if out.contains("ip link delete dev") {
        eprintln!("wg-quick up dn42-{asn}: {out}");
    }
}

fn run_wg_down(asn: u32) {
    run_cmd("wg-quick", &["down", &format!("dn42-{asn}")]);
}

fn run_birdc_configure() {
    run_cmd("birdc", &["c"]);
}

fn run_cmd(bin: &str, args: &[&str]) -> String {
    Command::new(bin)
        .args(args)
        .output()
        .map(|o| {
            String::from_utf8_lossy(if o.status.success() {
                &o.stdout
            } else {
                &o.stderr
            })
            .into_owned()
        })
        .unwrap_or_default()
}

// --- helpers ---

fn parse_asn_header(headers: &HeaderMap) -> Result<u32, StatusCode> {
    headers
        .get("asn")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .ok_or(StatusCode::BAD_REQUEST)
}
