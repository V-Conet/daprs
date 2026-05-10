//! Agent 命令模块
//!
//! 提供网络诊断命令（ping, traceroute, dig 等）的执行功能。

use std::net::{IpAddr, Ipv6Addr};

use axum::{Json, extract::State};

use crate::config::Config;
use crate::utils::run_cmd;
use shared::{AppError, Cmd, QueryType};

/// 禁止的字符（命令注入防护）
const FORBIDDEN_CHARS: &[char] = &[
    ';', '|', '&', '$', '`', '(', ')', '<', '>', '\n', '\r', '\t',
];

// 输入验证
fn validate_target(target: &str) -> Result<(), AppError> {
    if target.is_empty() || target.len() > 253 {
        return Err(bad("invalid target"));
    }
    if target
        .chars()
        .any(|c| FORBIDDEN_CHARS.contains(&c) || c == ' ')
        || target.starts_with('-')
    {
        return Err(bad("invalid target"));
    }
    // 纯 IP
    if target.parse::<IpAddr>().is_ok() {
        return Ok(());
    }
    // [IPv6]:port 格式
    if target.starts_with('[') && target.contains(']') {
        let close = target.find(']').unwrap();
        if target[1..close].parse::<Ipv6Addr>().is_ok() {
            return Ok(());
        }
    }
    // 纯 IPv6
    if target.contains(':') && target.parse::<Ipv6Addr>().is_ok() {
        return Ok(());
    }
    // 域名
    for label in target.split('.') {
        if label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label.chars().all(|c| c.is_alphanumeric() || c == '-')
        {
            return Err(bad("invalid domain"));
        }
    }
    Ok(())
}

fn validate_identifier(name: &str, max_len: usize) -> Result<(), AppError> {
    if name.is_empty()
        || name.len() > max_len
        || name.starts_with('-')
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(bad("invalid name"));
    }
    Ok(())
}

fn validate_server(server: &str) -> Result<(), AppError> {
    let server = server.trim_start_matches('@');
    if server
        .chars()
        .any(|c| FORBIDDEN_CHARS.contains(&c) || c == ' ' || c == '-')
    {
        return Err(bad("invalid server"));
    }
    validate_target(server)
}

fn bad(msg: &str) -> AppError {
    AppError::BadRequest(msg.into())
}

// API Handler
pub async fn cmd_handler(
    State(cfg): State<Config>,
    Json(cmd): Json<Cmd>,
) -> Result<String, AppError> {
    match cmd {
        Cmd::Ping {
            protocol,
            count,
            size,
            dfrag,
            timeout,
            target,
        } => handle_ping(&cfg, protocol, count, size, dfrag, timeout, target).await,
        Cmd::Traceroute { protocol, target } => handle_traceroute(&cfg, protocol, target).await,
        Cmd::Dig {
            qtype,
            server,
            target,
        } => handle_dig(&cfg, qtype, server, target).await,
        Cmd::WgShow { interface } => {
            validate_identifier(&interface, 15)?;
            exec_cmd(&cfg, "wg", &["show", &interface]).await
        }
        Cmd::BirdShow { protocol } => {
            validate_identifier(&protocol, 64)?;
            exec_cmd(&cfg, "birdc", &["show", "protocol", &protocol]).await
        }
    }
}

// Command Handlers
async fn handle_ping(
    cfg: &Config,
    protocol: Option<u16>,
    count: Option<u16>,
    size: Option<u16>,
    dfrag: Option<bool>,
    timeout_ms: Option<u32>,
    target: String,
) -> Result<String, AppError> {
    validate_target(&target)?;

    let mut args: Vec<String> = vec![
        "-c".into(),
        count.unwrap_or(4).to_string(),
        "-w".into(),
        timeout_ms.unwrap_or(2000).to_string(),
    ];

    if let Some(s) = size {
        args.extend(["-s".into(), s.to_string()]);
    }
    if dfrag.unwrap_or(false) {
        args.push("-F".into());
    }
    match protocol {
        Some(4) => args.push("-4".into()),
        Some(6) => args.push("-6".into()),
        Some(_) => return Err(bad("invalid protocol")),
        None => {}
    }
    args.push(target.to_lowercase());

    exec_cmd_strs(cfg, "ping", &args).await
}

async fn handle_traceroute(
    cfg: &Config,
    protocol: Option<u16>,
    target: String,
) -> Result<String, AppError> {
    validate_target(&target)?;

    let mut args: Vec<&str> = vec!["-q1", "-N32", "-w1"];
    match protocol {
        Some(4) => args.push("-4"),
        Some(6) => args.push("-6"),
        Some(_) => return Err(bad("invalid protocol")),
        None => {}
    }
    args.push(&target);

    // traceroute 经常返回非零退出码但仍有输出
    let output = run_cmd(cfg, "traceroute", &args).await;
    if output.success || !output.text.is_empty() {
        Ok(output.text)
    } else {
        Err(bad("traceroute failed"))
    }
}

async fn handle_dig(
    cfg: &Config,
    qtype: QueryType,
    server: Option<String>,
    target: String,
) -> Result<String, AppError> {
    validate_target(&target)?;

    let mut args: Vec<String> = vec![qtype.as_dig_arg().into()];
    if let Some(s) = server {
        validate_server(&s)?;
        args.push(format!("@{}", s.trim_start_matches('@')));
    }
    args.push(target.to_lowercase());

    exec_cmd_strs(cfg, "dig", &args).await
}

async fn exec_cmd(cfg: &Config, bin: &str, args: &[&str]) -> Result<String, AppError> {
    let output = run_cmd(cfg, bin, args).await;
    if output.success {
        Ok(output.text)
    } else {
        Err(AppError::BadRequest(output.text))
    }
}

async fn exec_cmd_strs(cfg: &Config, bin: &str, args: &[String]) -> Result<String, AppError> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    exec_cmd(cfg, bin, &args).await
}
