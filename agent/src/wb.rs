// Write Wireguard & Bird configuration files

use std::{fmt::Debug, process::Command};

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use toml::value::Time;

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

#[derive(Deserialize, Serialize, Debug)]
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

// --- handlers ---

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

    Ok(StatusCode::OK)
}

pub async fn delete_config(
    headers: HeaderMap,
    State(cfg): State<Config>,
) -> Result<StatusCode, StatusCode> {
    let asn = parse_asn_header(&headers)?;

    // Check file existence before running CMDs
    let wg_path = format!("{}/dn42-{}.conf", cfg.agent.wg_path, asn);
    let bird_path = format!("{}/dn42_peers/{}.conf", cfg.agent.bird_path, asn);

    let wg_existed = std::path::Path::new(&wg_path).exists();
    let bird_existed = std::path::Path::new(&bird_path).exists();

    if wg_existed {
        run_wg_down(asn);
    }
    remove_wg(&cfg, asn);
    remove_bird(&cfg, asn);

    if wg_existed || bird_existed {
        run_birdc_configure();
    }

    Ok(StatusCode::OK)
}
/// 查询 Peer 配置和状态信息，包括 wg/bird 配置和运行状态
pub async fn get_peer_info(
    headers: HeaderMap,
    State(cfg): State<Config>,
) -> Result<Json<PeerInfoResponse>, StatusCode> {
    let asn = parse_asn_header(&headers)?;

    // TODO: carefully handle file reading
    let wg_path = format!("{}/dn42-{}.conf", cfg.agent.wg_path, asn);
    let bird_path = format!("{}/dn42_peers/{}.conf", cfg.agent.bird_path, asn);

    let wg_info = parse_wg_config(&wg_path);
    let bird_info = parse_bird_config(&bird_path);

    let wg_status = run_cmd("wg", &["show", &format!("dn42-{asn}")]);
    let bird_status = run_bird_status(asn, &bird_info.sessions);

    Ok(Json(PeerInfoResponse {
        asn,
        port: wg_info.port,
        v4: wg_info.v4_peer,
        v6: wg_info.v6_peer,
        lla: wg_info.lla_peer,
        endpoint: wg_info.endpoint,
        pubkey: wg_info.pubkey,
        psk: wg_info.psk,
        mtu: wg_info.mtu,
        policy: bird_info.policy,
        is_mhp: bird_info.is_mhp,
        is_nhp: bird_info.is_nhp,
        is_prefer_lla: bird_info.is_prefer_lla,
        session_type: bird_info.session_type,
        wg_status: if wg_status.is_empty() {
            None
        } else {
            Some(wg_status)
        },
        bird_status: if bird_status.is_empty() {
            None
        } else {
            Some(bird_status)
        },
        my_v4: cfg.agent.dn42.ipv4.clone(),
        my_v6: cfg.agent.dn42.ipv6.clone(),
        my_lla: cfg.agent.dn42.lla.clone(),
        my_pubkey: cfg.agent.dn42.wgkey.clone(),
    }))
}

// --- response types ---

#[derive(Serialize)]
pub struct PeerInfoResponse {
    pub asn: u32,
    pub port: u16,
    pub v4: Option<String>,
    pub v6: Option<String>,
    pub lla: Option<String>,
    pub endpoint: Option<String>,
    pub pubkey: Option<String>,
    pub psk: Option<String>,
    pub mtu: Option<u16>,
    pub policy: Option<String>,
    pub is_mhp: bool,
    pub is_nhp: bool,
    pub is_prefer_lla: bool,
    pub session_type: Option<String>,
    pub wg_status: Option<String>,
    pub bird_status: Option<String>,
    pub my_v4: String,
    pub my_v6: String,
    pub my_lla: String,
    pub my_pubkey: String,
}

// --- config parsing ---

struct WgInfo {
    port: u16,
    v4_peer: Option<String>,
    v6_peer: Option<String>,
    lla_peer: Option<String>,
    endpoint: Option<String>,
    pubkey: Option<String>,
    psk: Option<String>,
    mtu: Option<u16>,
}

struct BirdInfo {
    policy: Option<String>,
    is_mhp: bool,
    is_nhp: bool,
    is_prefer_lla: bool,
    session_type: Option<String>,
    sessions: Vec<(u8, String)>, // (version, neighbor) for birdc queries
}

fn parse_wg_config(path: &str) -> WgInfo {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            return WgInfo {
                port: 0,
                v4_peer: None,
                v6_peer: None,
                lla_peer: None,
                endpoint: None,
                pubkey: None,
                psk: None,
                mtu: None,
            };
        }
    };

    let mut port = 0u16;
    let mut v4_peer = None;
    let mut v6_peer = None;
    let mut lla_peer = None;
    let mut endpoint = None;
    let mut pubkey = None;
    let mut psk = None;
    let mut mtu = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('[') {
            continue;
        }

        if let Some(val) = parse_after(line, "ListenPort = ") {
            port = val.parse().unwrap_or(0);
        } else if let Some(val) = parse_after(line, "MTU = ") {
            mtu = Some(val.parse().unwrap_or(1420));
        } else if let Some(val) = parse_after(line, "PublicKey = ") {
            pubkey = Some(val.to_string());
        } else if let Some(val) = parse_after(line, "PresharedKey = ") {
            psk = Some(val.to_string());
        } else if let Some(val) = parse_after(line, "Endpoint = ") {
            endpoint = Some(val.to_string());
        } else if line.starts_with("PostUp = ip addr add ") {
            // Format: PostUp = ip addr add {my_ip}/CIDR peer {peer_ip}/CIDR dev %i
            // Or no peer: PostUp = ip addr add {my_ip}/CIDR dev %i
            let rest = line.strip_prefix("PostUp = ip addr add ").unwrap();
            if let Some(peer_section) = rest.split(" peer ").nth(1) {
                // Extract IP before "/" in "peer {ip}/CIDR dev %i"
                let ip = peer_section.split('/').next().unwrap_or("");
                if ip.starts_with("fe80:") {
                    lla_peer = Some(ip.to_string());
                } else if ip.contains(':') {
                    v6_peer = Some(ip.to_string());
                } else if ip.contains('.') {
                    v4_peer = Some(ip.to_string());
                }
            }
        }
    }

    WgInfo {
        port,
        v4_peer,
        v6_peer,
        lla_peer,
        endpoint,
        pubkey,
        psk,
        mtu,
    }
}

fn parse_bird_config(path: &str) -> BirdInfo {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            return BirdInfo {
                policy: None,
                is_mhp: false,
                is_nhp: false,
                is_prefer_lla: false,
                session_type: None,
                sessions: vec![],
            };
        }
    };

    let mut policy = None;
    let mut is_mhp = false;
    let mut is_nhp = false;
    let mut is_prefer_lla = false;
    let mut versions_seen = vec![];
    let mut sessions = vec![];
    let mut has_block_ipv4 = false;
    let mut has_block_ipv6 = false;

    for line in content.lines() {
        let line = line.trim();

        // "protocol bgp DN42_{asn}_v{4|6} from dn42_peers {"
        if line.starts_with("protocol bgp DN42_") {
            if let Some(ver_str) = line
                .strip_prefix("protocol bgp DN42_")
                .and_then(|rest| rest.split_once('_'))
                .and_then(|(_, after)| after.strip_prefix('v'))
            {
                if let Some(ver) = ver_str
                    .split_whitespace()
                    .next()
                    .and_then(|v| v.parse::<u8>().ok())
                {
                    versions_seen.push(ver);
                }
            }
        }

        // neighbor line
        if let Some(neighbor) = parse_after(line, "neighbor ") {
            let neighbor = neighbor.split_whitespace().next().unwrap_or("").to_string();
            if let Some(&ver) = versions_seen.last() {
                sessions.push((ver, neighbor));
            }
        }

        // policy comment
        if line.starts_with("# policy: ") || line.starts_with("\t# policy: ") {
            let raw = line.split("# policy:").nth(1).unwrap_or("").trim();
            // strip the leading "{policy:?}" debug part
            if let Some(pol) = raw.split_whitespace().next() {
                if !pol.starts_with('(') {
                    policy = Some(pol.to_string());
                } else {
                    policy = Some(raw.to_string());
                }
            }
            if raw.contains("prefer LLA") {
                is_prefer_lla = true;
            }
        }

        // ipv4 { import none; export none; }
        if line == "ipv4 {" {
            has_block_ipv4 = true;
        }
        if line == "ipv6 {" {
            has_block_ipv6 = true;
        }
    }

    // Determine session type from versions seen and blocked AFs
    let session_type = if versions_seen.len() == 1 {
        let v = versions_seen[0];
        let has_both_blocked = has_block_ipv4 || has_block_ipv6;
        if has_both_blocked {
            // Single channel: one version peers but blocks the other AF
            if v == 6 {
                is_mhp = true;
                is_nhp = true; // v6 carrying v4 routes → ENH
                Some("mpbgp_v6".to_string())
            } else {
                is_mhp = true;
                Some("mpbgp_v4".to_string())
            }
        } else {
            Some(format!("v{v}"))
        }
    } else if versions_seen.len() == 2 {
        // Two separate sessions (no MP-BGP)
        Some("dual".to_string())
    } else {
        None
    };

    BirdInfo {
        policy,
        is_mhp,
        is_nhp,
        is_prefer_lla,
        session_type,
        sessions,
    }
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

fn run_bird_status(_asn: u32, sessions: &[(u8, String)]) -> String {
    let mut results = Vec::new();
    for (version, _neighbor) in sessions {
        let proto = format!("DN42_{}_v{}", _asn, version);
        let out = run_cmd("birdc", &["show", "protocols", &proto]);
        if !out.is_empty() {
            results.push(out);
        }
    }
    results.join("\n")
}


// FIXME: bird 控制需要使用 unix socket 连接到控制端口，以兼容容器运行
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

fn parse_after<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    line.strip_prefix(prefix).map(|s| s.trim())
}
