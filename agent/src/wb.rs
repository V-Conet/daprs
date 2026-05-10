//! WireGuard & Bird 配置管理模块
//!
//! 负责 WireGuard 和 Bird 的配置生成与管理

use std::path::Path;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};

use crate::config::Config;
use crate::utils::{parse_asn_header, run_cmd};
use shared::{
    AppError, BirdConfig, PeerInfo, PeerInfoResponse, PeeringPayload, RawCommandOutput, WgConfig,
};

// API Handlers
/// 创建 Peer 配置
///
/// 生成 WireGuard 配置文件和 Bird BGP 配置文件，并启动隧道
pub async fn create_config(
    headers: HeaderMap,
    State(cfg): State<Config>,
    Json(peer): Json<PeeringPayload>,
) -> Result<StatusCode, AppError> {
    let asn = parse_asn_header(&headers)?;

    // 输入验证
    validate_peering_payload(&peer)?;

    // 检查是否已存在
    let wg_path = format!("{}/dn42-{}.conf", cfg.agent.wg_path, asn);
    let bird_path = format!("{}/{}.conf", cfg.agent.bird_path, asn);

    if Path::new(&wg_path).exists() || Path::new(&bird_path).exists() {
        return Err(AppError::BadRequest(format!(
            "peer {} already exists, use modify instead",
            asn
        )));
    }

    // 生成配置
    let wg_conf = build_wg_config(asn, &peer, &cfg);
    let bird_conf = build_bird_config(asn, &peer);

    // 写入配置文件
    write_wg(&cfg, asn, &wg_conf)?;
    write_bird(&cfg, asn, &bird_conf)?;

    // 启动 WireGuard 接口
    run_wg_quick_up(&cfg, asn).await;

    // 重载 Bird 配置
    run_birdc_configure(&cfg).await;

    Ok(StatusCode::CREATED)
}

/// 修改 Peer 配置
///
/// 更新已有的 WireGuard 和 Bird 配置
pub async fn modify_config(
    headers: HeaderMap,
    State(cfg): State<Config>,
    Json(peer): Json<PeeringPayload>,
) -> Result<StatusCode, AppError> {
    let asn = parse_asn_header(&headers)?;

    // 输入验证
    validate_peering_payload(&peer)?;

    let wg_conf = build_wg_config(asn, &peer, &cfg);
    let bird_conf = build_bird_config(asn, &peer);

    write_wg(&cfg, asn, &wg_conf)?;
    write_bird(&cfg, asn, &bird_conf)?;

    // 重启 WireGuard 接口
    run_wg_quick_down(&cfg, asn).await;
    run_wg_quick_up(&cfg, asn).await;

    // 重载 Bird 配置
    run_birdc_configure(&cfg).await;

    Ok(StatusCode::OK)
}

/// 删除 Peer 配置
///
/// 删除 WireGuard 和 Bird 配置文件，并关闭隧道
pub async fn delete_config(
    headers: HeaderMap,
    State(cfg): State<Config>,
) -> Result<StatusCode, AppError> {
    let asn = parse_asn_header(&headers)?;

    let wg_path = format!("{}/dn42-{}.conf", cfg.agent.wg_path, asn);
    let bird_path = format!("{}/{}.conf", cfg.agent.bird_path, asn);

    let wg_existed = Path::new(&wg_path).exists();
    let bird_existed = Path::new(&bird_path).exists();

    // 关闭 WireGuard 接口
    if wg_existed {
        run_wg_quick_down(&cfg, asn).await;
    }

    // 删除配置文件
    remove_wg(&cfg, asn);
    remove_bird(&cfg, asn);

    // 重载 Bird 配置
    if wg_existed || bird_existed {
        run_birdc_configure(&cfg).await;
    }

    Ok(StatusCode::OK)
}

/// 查询 Peer 信息
///
/// 返回 WireGuard 接口状态、Bird 协议状态和配置信息
pub async fn get_peer_info(
    headers: HeaderMap,
    State(cfg): State<Config>,
) -> Result<Json<PeerInfoResponse>, AppError> {
    let asn = parse_asn_header(&headers)?;

    let wg_interface = format!("dn42-{asn}");
    let wg_path = format!("{}/dn42-{}.conf", cfg.agent.wg_path, asn);
    let bird_path = format!("{}/{}.conf", cfg.agent.bird_path, asn);

    // 尝试解析配置文件
    let wg_config = parse_wg_config(&wg_path).ok();
    let bird_config = parse_bird_config(&bird_path).ok();

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
        let active_protocols = get_active_bird_protocols(asn, &bird_path);
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

// 配置生成
/// 生成 WireGuard 配置
fn build_wg_config(asn: u32, peer: &PeeringPayload, cfg: &Config) -> String {
    // 使用请求中的 LLA，否则使用配置中的
    let my_lla = peer.lla.as_deref().unwrap_or(&cfg.agent.dn42.lla);
    let my_ula = &cfg.agent.dn42.ipv6;
    let my_v4 = &cfg.agent.dn42.ipv4;

    // 计算端口：DN42 ASN 使用后5位，Clearnet ASN 使用 40000 + 后4位
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
        pubkey = peer.pubkey,
        endpoint = peer.endpoint,
        psk = psk_line,
    )
}

/// 分类 Peer IP 地址
fn classify_peer_ips(peer: &PeeringPayload) -> (String, String, String) {
    // 链路本地地址（fe80::）
    let ll = peer
        .v6
        .as_ref()
        .filter(|v6| v6.starts_with("fe80:"))
        .map(|v6| format!(" peer {v6}/64"));

    // ULA 地址（非 fe80:: 的 IPv6）
    let ula = peer
        .v6
        .as_ref()
        .filter(|v6| !v6.starts_with("fe80:"))
        .map(|v6| format!(" peer {v6}/128"));

    // IPv4 地址
    let v4 = peer.v4.as_ref().map(|v4| format!(" peer {v4}/32"));

    (
        ll.unwrap_or_default(),
        ula.unwrap_or_default(),
        v4.unwrap_or_default(),
    )
}

/// 生成 Bird BGP 配置
///
/// 在此假设用户已全局启用 Nexthop
///
/// 配置逻辑：
/// - MP-BGP 启用：单个 BGP 会话传输两种协议
///   - is_nhp=true: IPv6 会话 (Extended Next Hop，默认开启)
///   - is_nhp=false: IPv4 会话
/// - MP-BGP 不启用：两个独立 BGP 会话，各自只传输自己的协议
/// - 单栈：单个 BGP 会话
fn build_bird_config(asn: u32, peer: &PeeringPayload) -> String {
    let has_v4 = peer.v4.is_some();
    let has_v6 = peer.v6.is_some();

    match (has_v6, has_v4) {
        // 双栈 + MP-BGP: 单个会话传输两种协议
        (true, true) if peer.is_mhp => {
            if peer.is_nhp {
                // IPv6 会话传输 IPv4 和 IPv6 路由 (Extended Next Hop)
                gen_bird_protocol(asn, 6, false, peer.v6.as_deref())
            } else {
                // IPv4 会话传输 IPv4 和 IPv6 路由
                gen_bird_protocol(asn, 4, false, peer.v4.as_deref())
            }
        }
        // 双栈 + 非 MP-BGP: 两个独立会话，各传输自己的协议
        (true, true) => {
            format!(
                "{}\n{}",
                // IPv6 会话只传输 IPv6
                gen_bird_protocol(asn, 6, true, peer.v6.as_deref()),
                // IPv4 会话只传输 IPv4
                gen_bird_protocol(asn, 4, true, peer.v4.as_deref()),
            )
        }
        // 仅 IPv6：单个会话
        (true, false) => gen_bird_protocol(asn, 6, false, peer.v6.as_deref()),
        // 仅 IPv4：单个会话
        (false, true) => gen_bird_protocol(asn, 4, false, peer.v4.as_deref()),
        // 无地址
        (false, false) => String::new(),
    }
}

/// 生成单个 Bird BGP 协议配置
///
/// # 参数
/// - `asn`: ASN 号码
/// - `version`: BGP 会话版本 (4 或 6)
/// - `block_other_af`: 是否 block 另一个地址族
///   - true: 单协议会话，block 另一个 AF
///   - false: MP-BGP 会话，传输两种协议
/// - `neighbor`: 邻居地址
fn gen_bird_protocol(
    asn: u32,
    version: u8,
    block_other_af: bool,
    neighbor: Option<&str>,
) -> String {
    let neighbor = neighbor.unwrap_or("");

    // IPv6 使用链路本地地址需要指定接口
    let neighbor_line = if version == 6 {
        format!("neighbor {} % 'dn42-{}' external;", neighbor, asn)
    } else {
        format!("neighbor {} external;", neighbor)
    };

    let mut text = format!(
        "protocol bgp DN42_{asn}_v{version} from dn42_peers {{\n\
         \x20   {neighbor_line}\n"
    );

    // IPv4 会话需要 direct（因为不使用链路本地地址）
    if version == 4 {
        text.push_str("    direct;\n");
    }

    // block 另一个地址族（单协议会话）
    if block_other_af {
        let other_af = if version == 6 { 4 } else { 6 };
        text.push_str(&format!(
            "    ipv{other_af} {{\n\
             \x20       import none;\n\
             \x20       export none;\n\
             \x20   }};\n"
        ));
    }

    text.push_str("}\n");
    text
}

/// 写入 WireGuard 配置文件
fn write_wg(cfg: &Config, asn: u32, conf: &str) -> Result<(), AppError> {
    let path = format!("{}/dn42-{}.conf", cfg.agent.wg_path, asn);
    std::fs::write(&path, conf).map_err(|e| {
        eprintln!("wg write error {path}: {e}");
        AppError::InternalError(format!("failed to write wg config: {e}"))
    })
}

/// 写入 Bird 配置文件
fn write_bird(cfg: &Config, asn: u32, conf: &str) -> Result<(), AppError> {
    let dir = format!("{}", cfg.agent.bird_path);
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::InternalError(format!("failed to create bird peers dir: {e}")))?;

    let path = format!("{}/{}.conf", dir, asn);
    std::fs::write(&path, conf).map_err(|e| {
        eprintln!("bird write error {path}: {e}");
        AppError::InternalError(format!("failed to write bird config: {e}"))
    })
}

/// 删除 WireGuard 配置文件
///
/// 如果文件不存在，静默返回（幂等操作）
fn remove_wg(cfg: &Config, asn: u32) {
    let path = format!("{}/dn42-{}.conf", cfg.agent.wg_path, asn);
    if std::path::Path::new(&path).exists() {
        if let Err(e) = std::fs::remove_file(&path) {
            eprintln!("wg remove error {path}: {e}");
        }
    }
}

/// 删除 Bird 配置文件
///
/// 如果文件不存在，静默返回（幂等操作）
fn remove_bird(cfg: &Config, asn: u32) {
    let path = format!("{}/{}.conf", cfg.agent.bird_path, asn);
    if std::path::Path::new(&path).exists() {
        if let Err(e) = std::fs::remove_file(&path) {
            eprintln!("bird remove error {path}: {e}");
        }
    }
}

/// 启动 WireGuard 接口
async fn run_wg_quick_up(cfg: &Config, asn: u32) {
    run_cmd(cfg, "wg-quick", &["up", &format!("dn42-{asn}")]).await;
}

/// 关闭 WireGuard 接口
async fn run_wg_quick_down(cfg: &Config, asn: u32) {
    run_cmd(cfg, "wg-quick", &["down", &format!("dn42-{asn}")]).await;
}

/// 重载 Bird 配置
async fn run_birdc_configure(cfg: &Config) {
    run_cmd(cfg, "birdc", &["c"]).await;
}

/// 获取实际存在的 Bird 协议名称
///
/// 解析 Bird 配置文件，返回实际定义的协议名称
fn get_active_bird_protocols(asn: u32, bird_path: &str) -> Vec<String> {
    let mut protocols = Vec::new();

    if let Ok(content) = std::fs::read_to_string(bird_path) {
        // 检查是否存在 v6 会话
        if content.contains(&format!("protocol bgp DN42_{asn}_v6")) {
            protocols.push(format!("DN42_{asn}_v6"));
        }
        // 检查是否存在 v4 会话
        if content.contains(&format!("protocol bgp DN42_{asn}_v4")) {
            protocols.push(format!("DN42_{asn}_v4"));
        }
    }

    protocols
}

/// 解析 WireGuard 配置文件
fn parse_wg_config(path: &str) -> Result<WgConfig, AppError> {
    let content = std::fs::read_to_string(path).map_err(|_| AppError::NotFound)?;

    let mut port: u16 = 0;
    let mut mtu: u16 = 1420;
    let mut pubkey = String::new();
    let mut psk: Option<String> = None;
    let mut endpoint: Option<String> = None;
    let mut peer_v4: Option<String> = None;
    let mut peer_v6: Option<String> = None;

    // 解析 [Interface] 部分
    let mut in_interface = false;
    let mut in_peer = false;

    for line in content.lines() {
        let line = line.trim();

        if line == "[Interface]" {
            in_interface = true;
            in_peer = false;
            continue;
        }
        if line == "[Peer]" {
            in_interface = false;
            in_peer = true;
            continue;
        }

        if in_interface {
            if let Some(value) = line.strip_prefix("ListenPort = ") {
                port = value.parse().unwrap_or(0);
            } else if let Some(value) = line.strip_prefix("MTU = ") {
                mtu = value.parse().unwrap_or(1420);
            } else if line.starts_with("PostUp = ip addr add") {
                // 解析对端 IP 地址
                // 格式: PostUp = ip addr add xxx/xx peer yyy/xx dev %i
                if let Some(peer_match) = line.split(" peer ").nth(1) {
                    let addr = peer_match.split('/').next().unwrap_or("").trim();
                    if addr.starts_with("fe80:") {
                        peer_v6 = Some(addr.to_string());
                    } else if addr.starts_with("fd") || addr.starts_with("fc") {
                        peer_v6 = Some(addr.to_string());
                    } else if addr.contains('.') {
                        peer_v4 = Some(addr.to_string());
                    }
                }
            }
        }

        if in_peer {
            if let Some(value) = line.strip_prefix("PublicKey = ") {
                pubkey = value.to_string();
            } else if let Some(value) = line.strip_prefix("PresharedKey = ") {
                psk = Some(value.to_string());
            } else if let Some(value) = line.strip_prefix("Endpoint = ") {
                endpoint = Some(value.to_string());
            }
        }
    }

    Ok(WgConfig {
        port,
        mtu,
        pubkey,
        psk,
        endpoint,
        peer_v4,
        peer_v6,
    })
}

/// 解析 Bird 配置文件
fn parse_bird_config(path: &str) -> Result<BirdConfig, AppError> {
    let content = std::fs::read_to_string(path).map_err(|_| AppError::NotFound)?;

    let mut has_v4_session = false;
    let mut has_v6_session = false;
    let mut v4_blocked = false;
    let mut v6_blocked = false;

    // 检查是否有 v4 和 v6 会话
    if content.contains(&format!("protocol bgp DN42_")) {
        // 简单解析：检查是否存在 ipv4/ipv6 block
        if content.contains("ipv4 {") {
            v4_blocked = content.contains("ipv4 {") && content.contains("import none;");
        }
        if content.contains("ipv6 {") {
            v6_blocked = content.contains("ipv6 {") && content.contains("import none;");
        }

        // 检查会话类型
        if content.contains("_v4 from") {
            has_v4_session = true;
        }
        if content.contains("_v6 from") {
            has_v6_session = true;
        }
    }

    // 确定配置类型
    let (is_mhp, is_nhp, session_type) =
        match (has_v6_session, has_v4_session, v4_blocked, v6_blocked) {
            // 只有 v6 会话，v4 被 block -> MP-BGP over IPv6 (ENH)
            (true, false, true, false) => (
                true,
                true,
                "MP-BGP over IPv6 (Extended Next Hop)".to_string(),
            ),
            // 只有 v4 会话，v6 被 block -> MP-BGP over IPv4
            (false, true, false, true) => (true, false, "MP-BGP over IPv4".to_string()),
            // 两个会话都有，各自 block 对方 -> 双会话
            (true, true, true, true) => (false, false, "Dual BGP Sessions".to_string()),
            // 只有 v6 会话，无 block -> 单栈 IPv6
            (true, false, false, false) => (false, false, "IPv6 Only".to_string()),
            // 只有 v4 会话，无 block -> 单栈 IPv4
            (false, true, false, false) => (false, false, "IPv4 Only".to_string()),
            // 其他情况
            _ => (false, false, "Unknown".to_string()),
        };

    Ok(BirdConfig {
        is_mhp,
        is_nhp,
        session_type,
    })
}

// 输入验证
/// 验证 Peering 请求参数
fn validate_peering_payload(peer: &PeeringPayload) -> Result<(), AppError> {
    // 验证公钥格式 (WireGuard 公钥为 44 字符 base64)
    if peer.pubkey.len() != 44 || !is_valid_base64(&peer.pubkey) {
        return Err(AppError::BadRequest("invalid public key format".into()));
    }

    // 验证 endpoint 长度和格式
    if peer.endpoint.is_empty() || peer.endpoint.len() > 256 {
        return Err(AppError::BadRequest("invalid endpoint".into()));
    }
    // 防止注入攻击：endpoint 不应包含特殊字符
    if peer
        .endpoint
        .contains(|c: char| c == '\n' || c == '\r' || c == '\0' || c == '"')
    {
        return Err(AppError::BadRequest(
            "invalid characters in endpoint".into(),
        ));
    }

    // 验证预共享密钥（如果提供）
    if let Some(ref psk) = peer.psk {
        if psk.len() != 44 || !is_valid_base64(psk) {
            return Err(AppError::BadRequest("invalid preshared key format".into()));
        }
    }

    // 验证 IPv4 地址格式（如果提供）
    if let Some(ref v4) = peer.v4 {
        if !is_valid_ipv4(v4) {
            return Err(AppError::BadRequest("invalid IPv4 address".into()));
        }
    }

    // 验证 IPv6 地址格式（如果提供）
    if let Some(ref v6) = peer.v6 {
        if !is_valid_ipv6(v6) {
            return Err(AppError::BadRequest("invalid IPv6 address".into()));
        }
    }

    // 验证 MTU 范围
    if let Some(mtu) = peer.mtu {
        if mtu < 576 || mtu > 9000 {
            return Err(AppError::BadRequest(
                "MTU must be between 576 and 9000".into(),
            ));
        }
    }

    // 验证端口范围
    if let Some(port) = peer.custom_port {
        if port < 1024 {
            return Err(AppError::BadRequest(
                "port must be between 1024 and 65535".into(),
            ));
        }
    }

    Ok(())
}

/// 检查是否为有效的 base64 字符串
fn is_valid_base64(s: &str) -> bool {
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
}

/// 检查是否为有效的 IPv4 地址
fn is_valid_ipv4(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    parts.iter().all(|p| p.parse::<u8>().is_ok())
}

/// 检查是否为有效的 IPv6 地址（简化检查）
fn is_valid_ipv6(s: &str) -> bool {
    // 简化检查：长度和字符范围
    if s.is_empty() || s.len() > 45 {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_hexdigit() || c == ':' || c == '.')
}
