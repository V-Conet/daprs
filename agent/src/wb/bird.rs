//! Bird BGP 配置生成、文件 IO 与 birdc 控制

use std::time::Duration;

use shared::{AppError, BirdConfig, PeeringPayload};
use tokio::time::sleep;

use crate::config::Config;
use crate::utils::run_cmd;

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
pub(super) fn build_bird_config(asn: u32, peer: &PeeringPayload) -> String {
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

/// 写入 Bird 配置文件
pub(super) fn write_bird(cfg: &Config, asn: u32, conf: &str) -> Result<(), AppError> {
    let dir = cfg.agent.bird_path.to_string();
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::InternalError(format!("failed to create bird peers dir: {e}")))?;

    let path = format!("{}/{}.conf", dir, asn);
    std::fs::write(&path, conf)
        .map_err(|e| AppError::InternalError(format!("failed to write bird config: {e}")))
}

/// 删除 Bird 配置文件
///
/// 如果文件不存在，静默返回（幂等操作）
pub(super) fn remove_bird(cfg: &Config, asn: u32) {
    let path = format!("{}/{}.conf", cfg.agent.bird_path, asn);
    if std::path::Path::new(&path).exists()
        && let Err(e) = std::fs::remove_file(&path)
    {
        tracing::warn!("bird remove error {path}: {e}");
    }
}

/// Bird 配置文件路径
pub(super) fn bird_config_path(cfg: &Config, asn: u32) -> String {
    format!("{}/{}.conf", cfg.agent.bird_path, asn)
}

/// 重载 Bird 配置
///
/// 使用 `birdc configure` 重载配置，带重试机制
pub(super) async fn run_birdc_configure(cfg: &Config) -> Result<(), AppError> {
    for attempt in 1..=3 {
        let result = run_cmd(cfg, "birdc", &["configure"]).await;

        if result.success {
            return Ok(());
        }

        tracing::warn!("birdc configure attempt {attempt} failed: {}", result.text);
        if attempt < 3 {
            sleep(Duration::from_millis(200)).await;
        }
    }

    Err(AppError::InternalError("birdc configure failed".into()))
}

/// 获取实际存在的 Bird 协议名称
///
/// 解析 Bird 配置文件，返回实际定义的协议名称
pub(super) fn get_active_bird_protocols(asn: u32, bird_path: &str) -> Vec<String> {
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

/// 解析 Bird 配置文件
pub(super) fn parse_bird_config(path: &str) -> Result<BirdConfig, AppError> {
    let content = std::fs::read_to_string(path).map_err(|_| AppError::NotFound)?;

    // 检查是否有 v4 和 v6 会话
    let has_v4_session = content.contains("protocol bgp DN42_") && content.contains("_v4 from");
    let has_v6_session = content.contains("protocol bgp DN42_") && content.contains("_v6 from");

    // 检查每个会话是否 block 了另一个地址族
    let v4_blocks_v6 = has_v4_session
        && content.contains("_v4 from")
        && content
            .lines()
            .skip_while(|l| !l.contains("_v4 from"))
            .take_while(|l| !l.contains("}"))
            .any(|l| l.contains("ipv6 {"));
    let v6_blocks_v4 = has_v6_session
        && content.contains("_v6 from")
        && content
            .lines()
            .skip_while(|l| !l.contains("_v6 from"))
            .take_while(|l| !l.contains("}"))
            .any(|l| l.contains("ipv4 {"));

    // 确定配置类型
    let (is_mhp, is_nhp, session_type) =
        match (has_v6_session, has_v4_session, v6_blocks_v4, v4_blocks_v6) {
            // 只有 v6 会话，block 了 v4 -> MP-BGP over IPv6 (ENH)
            (true, false, true, false) => (true, true, "MP-BGP over IPv6 (ENH)".into()),
            // 只有 v4 会话，block 了 v6 -> MP-BGP over IPv4
            (false, true, false, true) => (true, false, "MP-BGP over IPv4".into()),
            // 两个会话都有，各自 block 对方 -> Dual Sessions
            (true, true, true, true) => (false, false, "Dual BGP Sessions".into()),
            // 只有 v6 会话，无 block -> IPv6 Only
            (true, false, false, false) => (false, false, "IPv6 Only".into()),
            // 只有 v4 会话，无 block -> IPv4 Only
            (false, true, false, false) => (false, false, "IPv4 Only".into()),
            // 其他情况
            _ => (false, false, "Unknown".into()),
        };

    Ok(BirdConfig {
        is_mhp,
        is_nhp,
        session_type,
    })
}
