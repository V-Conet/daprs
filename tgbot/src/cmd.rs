//! Tgbot 命令处理模块
//!
//! 提供可扩展的网络诊断命令处理功能。

use std::collections::HashMap;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::{
    requests::Requester,
    types::{Message, ParseMode, ReplyParameters},
    utils::command::BotCommands,
};
use tokio::sync::Mutex;

use crate::agent::AgentClient;
use crate::config::Config;
use crate::{CacheEntry, NodeResult, build_keyboard_with_id, format_node_result, gen_id};

type CacheMap = Arc<Mutex<HashMap<String, CacheEntry>>>;

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    #[command(description = "Show help message")]
    Help,
    #[command(description = "Ping a target")]
    Ping,
    #[command(description = "Traceroute to target")]
    #[command(aliases = ["traceroute"])]
    Trace,
    #[command(description = "DNS lookup (usage: /dig <domain> [@server] [type])")]
    Dig,
    #[command(description = "TCP ping (usage: /tcping <host> <port>)")]
    TcPing,
    #[command(description = "Show route to target")]
    Route,
    #[command(description = "Show AS path to target")]
    Path,
}

pub async fn answer(
    bot: Bot,
    msg: Message,
    cmd: Command,
    config: Config,
    cache: CacheMap,
) -> ResponseResult<()> {
    let agent = AgentClient::new(config);

    match cmd {
        Command::Help => help(&bot, &msg).await?,
        Command::Ping => handle_ping(&bot, &msg, &agent, &cache).await?,
        Command::Trace => handle_trace(&bot, &msg, &agent, &cache).await?,
        Command::Dig => handle_dig(&bot, &msg, &agent, &cache).await?,
        Command::TcPing => handle_tcping(&bot, &msg, &agent, &cache).await?,
        Command::Route => handle_route(&bot, &msg, &agent, &cache).await?,
        Command::Path => handle_path(&bot, &msg, &agent, &cache).await?,
    }

    Ok(())
}

async fn help(bot: &Bot, msg: &Message) -> ResponseResult<()> {
    let text = r#"🤖 DN42 Network Tools Bot

Available Commands:
/ping <target> - Ping a target
/trace <target> - Traceroute to target
/dig <domain> [@server] [type] - DNS lookup
/tcping <host> <port> - TCP ping
/route <target> - Show route
/path <target> - Show AS path

Commands are executed on all nodes. Use buttons to switch between results."#;
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

fn parse_args(text: &str, min_args: usize) -> Option<Vec<String>> {
    let parts: Vec<String> = text
        .split_whitespace()
        .skip(1)
        .map(|s| s.to_string())
        .collect();
    if parts.len() >= min_args {
        Some(parts)
    } else {
        None
    }
}

fn get_all_nodes(agent: &AgentClient) -> Vec<String> {
    agent.nodes().iter().map(|n| n.name.clone()).collect()
}

async fn execute_all(
    agent: &AgentClient,
    nodes: &[String],
    make_cmd: impl Fn() -> shared::Cmd + Send + Sync,
) -> Vec<NodeResult> {
    let mut results = Vec::new();
    for node in nodes {
        let cmd = make_cmd();
        let result = agent.execute(node, cmd).await;
        results.push(NodeResult {
            node: node.clone(),
            output: result,
        });
    }
    results
}

/// 统一的命令执行和回复函数
///
/// 发送 placeholder 消息（引用用户消息），执行命令后更新该消息
async fn execute_and_reply<F>(
    bot: &Bot,
    msg: &Message,
    agent: &AgentClient,
    cache: &CacheMap,
    placeholder_text: &str,
    target: &str,
    make_cmd: F,
) -> ResponseResult<()>
where
    F: Fn() -> shared::Cmd + Send + Sync,
{
    let nodes = get_all_nodes(agent);

    if nodes.is_empty() {
        bot.send_message(msg.chat.id, "No nodes configured").await?;
        return Ok(());
    }

    // 1. 发送 placeholder 消息，引用用户消息
    let placeholder = bot
        .send_message(msg.chat.id, placeholder_text)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;

    // 2. 执行命令
    let results = execute_all(agent, &nodes, make_cmd).await;

    // 3. 缓存结果用于节点切换
    let cache_id = gen_id();
    let entry = CacheEntry {
        results: results.clone(),
        nodes: nodes.clone(),
        cmd_name: String::new(),
        target: target.to_string(),
    };

    {
        let mut map = cache.lock().await;
        map.insert(cache_id.clone(), entry);
    }

    // 4. 格式化第一个节点的结果
    let text = format_node_result(&results[0]);
    let keyboard = build_keyboard_with_id(&cache_id, &nodes, 0);

    // 5. 更新 placeholder 消息（而不是删除重发）
    bot.edit_message_text(msg.chat.id, placeholder.id, &text)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

async fn handle_ping(
    bot: &Bot,
    msg: &Message,
    agent: &AgentClient,
    cache: &CacheMap,
) -> ResponseResult<()> {
    let text = msg.text().unwrap_or("");
    let args = match parse_args(text, 1) {
        Some(a) => a,
        None => {
            bot.send_message(msg.chat.id, "Usage: /ping <target>")
                .await?;
            return Ok(());
        }
    };

    let target = args[0].clone();
    let placeholder_text = format!("⏳ Pinging {}...", target);

    execute_and_reply(bot, msg, agent, cache, &placeholder_text, &target, || {
        shared::Cmd::Ping {
            target: target.clone(),
            count: Some(4),
            protocol: None,
            size: None,
            dfrag: None,
            timeout: None,
        }
    })
    .await
}

async fn handle_trace(
    bot: &Bot,
    msg: &Message,
    agent: &AgentClient,
    cache: &CacheMap,
) -> ResponseResult<()> {
    let text = msg.text().unwrap_or("");
    let args = match parse_args(text, 1) {
        Some(a) => a,
        None => {
            bot.send_message(msg.chat.id, "Usage: /trace <target>")
                .await?;
            return Ok(());
        }
    };

    let target = args[0].clone();
    let placeholder_text = format!("⏳ Traceroute to {}...", target);

    execute_and_reply(bot, msg, agent, cache, &placeholder_text, &target, || {
        shared::Cmd::Traceroute {
            target: target.clone(),
            protocol: None,
        }
    })
    .await
}

async fn handle_dig(
    bot: &Bot,
    msg: &Message,
    agent: &AgentClient,
    cache: &CacheMap,
) -> ResponseResult<()> {
    let text = msg.text().unwrap_or("");
    let args = match parse_args(text, 1) {
        Some(a) => a,
        None => {
            bot.send_message(msg.chat.id, "Usage: /dig <domain> [@server] [type]")
                .await?;
            return Ok(());
        }
    };

    let target = args[0].clone();
    let mut server: Option<String> = None;
    let mut qtype_str = "A";

    for arg in args.iter().skip(1) {
        if let Some(stripped) = arg.strip_prefix('@') {
            server = Some(stripped.to_string());
        } else {
            qtype_str = arg;
        }
    }

    let qtype = match qtype_str {
        "A" => shared::QueryType::A,
        "AAAA" => shared::QueryType::AAAA,
        "MX" => shared::QueryType::MX,
        "TXT" => shared::QueryType::TXT,
        "NS" => shared::QueryType::NS,
        "SOA" => shared::QueryType::SOA,
        "CNAME" => shared::QueryType::CNAME,
        "PTR" => shared::QueryType::PTR,
        _ => shared::QueryType::A,
    };

    let server_info = server.as_deref().unwrap_or("default");
    let placeholder_text = format!(
        "⏳ DNS lookup for {} (type: {}, server: {})...",
        target, qtype_str, server_info
    );

    execute_and_reply(
        bot,
        msg,
        agent,
        cache,
        &placeholder_text,
        &format!("{} ({})", target, qtype_str),
        || shared::Cmd::Dig {
            target: target.clone(),
            qtype: qtype.clone(),
            server: server.clone(),
        },
    )
    .await
}

async fn handle_tcping(
    bot: &Bot,
    msg: &Message,
    agent: &AgentClient,
    cache: &CacheMap,
) -> ResponseResult<()> {
    let text = msg.text().unwrap_or("");
    let args = match parse_args(text, 2) {
        Some(a) => a,
        None => {
            bot.send_message(msg.chat.id, "Usage: /tcping <host> <port>")
                .await?;
            return Ok(());
        }
    };

    let target = args[0].clone();
    let port: u16 = args[1].parse().unwrap_or(80);
    let placeholder_text = format!("⏳ TCP ping to {}:{}...", target, port);

    execute_and_reply(
        bot,
        msg,
        agent,
        cache,
        &placeholder_text,
        &format!("{}:{}", target, port),
        || shared::Cmd::TcPing {
            target: target.clone(),
            port,
            count: Some(5),
            timeout: Some(3),
            protocol: None,
        },
    )
    .await
}

async fn handle_route(
    bot: &Bot,
    msg: &Message,
    agent: &AgentClient,
    cache: &CacheMap,
) -> ResponseResult<()> {
    let text = msg.text().unwrap_or("");
    let args = match parse_args(text, 1) {
        Some(a) => a,
        None => {
            bot.send_message(msg.chat.id, "Usage: /route <target>")
                .await?;
            return Ok(());
        }
    };

    let target = args[0].clone();
    let placeholder_text = format!("⏳ Looking up route to {}...", target);

    execute_and_reply(bot, msg, agent, cache, &placeholder_text, &target, || {
        shared::Cmd::Route {
            target: target.clone(),
            protocol: None,
        }
    })
    .await
}

async fn handle_path(
    bot: &Bot,
    msg: &Message,
    agent: &AgentClient,
    cache: &CacheMap,
) -> ResponseResult<()> {
    let text = msg.text().unwrap_or("");
    let args = match parse_args(text, 1) {
        Some(a) => a,
        None => {
            bot.send_message(msg.chat.id, "Usage: /path <target>")
                .await?;
            return Ok(());
        }
    };

    let target = args[0].clone();
    let placeholder_text = format!("⏳ Looking up AS path to {}...", target);

    execute_and_reply(bot, msg, agent, cache, &placeholder_text, &target, || {
        shared::Cmd::Path {
            target: target.clone(),
            protocol: None,
        }
    })
    .await
}
