//! WireGuard 配置生成、文件 IO 与接口管理

use std::path::Path;

use shared::{AppError, PeeringPayload, WgConfig, validation::AsnType};

use crate::config::Config;
use crate::utils::run_cmd;

/// 生成 WireGuard 配置
pub(super) fn build_wg_config(asn: u32, peer: &PeeringPayload, cfg: &Config) -> String {
    // 使用请求中的 LLA，否则使用配置中的
    let my_lla = peer.lla.as_deref().unwrap_or(&cfg.agent.dn42.lla);
    let my_ula = &cfg.agent.dn42.ipv6;
    let my_v4 = &cfg.agent.dn42.ipv4;

    // 计算端口：DN42 ASN 使用后5位，Clearnet ASN 使用 40000 + 后4位
    let port = peer
        .custom_port
        .unwrap_or_else(|| AsnType::from_asn(asn).default_port(asn));
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

/// 写入 WireGuard 配置文件
pub(super) fn write_wg(cfg: &Config, asn: u32, conf: &str) -> Result<(), AppError> {
    let path = format!("{}/dn42-{}.conf", cfg.agent.wg_path, asn);
    write_secure_file(&path, conf)
        .map_err(|e| AppError::InternalError(format!("failed to write wg config: {e}")))
}

#[cfg(unix)]
fn write_secure_file(path: &str, content: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(content.as_bytes())?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn write_secure_file(path: &str, content: &str) -> std::io::Result<()> {
    std::fs::write(path, content)
}

/// 删除 WireGuard 配置文件
///
/// 如果文件不存在，静默返回（幂等操作）
pub(super) fn remove_wg(cfg: &Config, asn: u32) {
    let path = format!("{}/dn42-{}.conf", cfg.agent.wg_path, asn);
    if Path::new(&path).exists()
        && let Err(e) = std::fs::remove_file(&path)
    {
        tracing::warn!("wg remove error {path}: {e}");
    }
}

/// WireGuard 配置文件路径
pub(super) fn wg_config_path(cfg: &Config, asn: u32) -> String {
    format!("{}/dn42-{}.conf", cfg.agent.wg_path, asn)
}

/// 启动 WireGuard 接口
pub(super) async fn run_wg_quick_up(cfg: &Config, asn: u32) -> Result<(), AppError> {
    let result = run_cmd(cfg, "wg-quick", &["up", &format!("dn42-{asn}")]).await;
    if result.success {
        Ok(())
    } else {
        tracing::error!("wg-quick up failed for ASN {asn}: {}", result.text);
        Err(AppError::InternalError("wg-quick up failed".into()))
    }
}

/// 关闭 WireGuard 接口
pub(super) async fn run_wg_quick_down(cfg: &Config, asn: u32) -> Result<(), AppError> {
    let result = run_cmd(cfg, "wg-quick", &["down", &format!("dn42-{asn}")]).await;
    if result.success {
        Ok(())
    } else {
        tracing::warn!("wg-quick down failed for ASN {asn}: {}", result.text);
        Err(AppError::InternalError("wg-quick down failed".into()))
    }
}

/// 解析 WireGuard 配置文件
pub(super) fn parse_wg_config(path: &str) -> Result<WgConfig, AppError> {
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
                    if addr.starts_with("fe80:") || addr.starts_with("fd") || addr.starts_with("fc")
                    {
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
