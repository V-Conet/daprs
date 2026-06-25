//! 命令路由、Trait 与注册表
//!
//! `Command` enum 是 teloxide 路由入口（无逻辑）；每个变体经 `Registry`
//! 映射到一个 `TgCommand` impl。加新命令 = enum 加变体 + 新建命令文件 +
//! `Registry::build()` 加一行 insert。

pub mod dig;
pub mod path;
pub mod peer;
pub mod ping;
pub mod route;
pub mod start;
pub mod tcping;
pub mod trace;

use std::collections::HashMap;
use std::mem::discriminant;
use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

use crate::agent::AgentClient;
use crate::cache::Cache;
use crate::error::ResponseResult;
use crate::flow;

// teloxide 命令路由入口
#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    #[command(description = "Show help message")]
    Help,
    #[command(description = "Start the bot")]
    Start,
    #[command(description = "Ping a target: /ping <target>")]
    Ping,
    #[command(description = "Traceroute to target: /trace <target>")]
    #[command(aliases = ["traceroute"])]
    Trace,
    #[command(description = "DNS lookup: /dig <domain> [@server] [type]")]
    Dig,
    #[command(description = "TCP ping: /tcping <host> <port>")]
    TcPing,
    #[command(description = "Show route to target: /route <target>")]
    Route,
    #[command(description = "Show AS path to target: /path <target>")]
    Path,
    #[command(description = "Peering info")]
    Peer,
}

/// 命令解析结果
pub enum ParseResult {
    /// 在所有节点执行 cmd，并 edit placeholder 展示结果
    Run {
        cmd: shared::Cmd,
        target: String,
        placeholder: String,
    },
    /// 直接回复静态文本（不调用 agent），用于 /start /peer 等
    Reply(String),
    /// 用法错误，直接回复文本（不调用 agent）
    Usage(String),
}

/// 一个诊断命令：从消息文本解析参数并构造 shared::Cmd。
///
/// `/help` 不是 TgCommand，由 [`dispatch`] 直接处理。
pub trait TgCommand: Send + Sync {
    fn parse(&self, text: &str) -> ParseResult;
}

/// 命令注册表：Command 变体 -> 处理器
pub struct Registry {
    map: HashMap<std::mem::Discriminant<Command>, Arc<dyn TgCommand>>,
}

impl Registry {
    /// 注册所有诊断命令
    pub fn build() -> Self {
        let mut map = HashMap::new();
        map.insert(
            discriminant(&Command::Start),
            Arc::new(start::Start) as Arc<dyn TgCommand>,
        );
        map.insert(discriminant(&Command::Ping), Arc::new(ping::Ping));
        map.insert(discriminant(&Command::Trace), Arc::new(trace::Trace));
        map.insert(discriminant(&Command::Dig), Arc::new(dig::Dig));
        map.insert(discriminant(&Command::TcPing), Arc::new(tcping::TcPing));
        map.insert(discriminant(&Command::Route), Arc::new(route::Route));
        map.insert(discriminant(&Command::Path), Arc::new(path::Path));
        map.insert(discriminant(&Command::Peer), Arc::new(peer::Peer));
        Self { map }
    }

    pub fn get(&self, cmd: &Command) -> Option<&Arc<dyn TgCommand>> {
        self.map.get(&discriminant(cmd))
    }
}

/// 命令分发
pub async fn dispatch(
    bot: Bot,
    msg: Message,
    cmd: Command,
    agent: Arc<AgentClient>,
    cache: Cache,
    registry: Arc<Registry>,
) -> ResponseResult<()> {
    if matches!(cmd, Command::Help) {
        // Command::descriptions() 实现 Display；用纯文本发送，
        // 避免 MarkdownV2 对 — . 等字符的转义要求
        let help = format!("🤖 DN42 Network Tools Bot\n\n{}", Command::descriptions());
        bot.send_message(msg.chat.id, help).await?;
        return Ok(());
    }

    let Some(handler) = registry.get(&cmd) else {
        return Ok(());
    };

    let text = msg.text().unwrap_or_default();
    match handler.parse(text) {
        ParseResult::Run {
            cmd,
            target,
            placeholder,
        } => {
            flow::run_command(&bot, &msg, &agent, &cache, placeholder, target, cmd).await?;
        }
        ParseResult::Reply(r) => {
            bot.send_message(msg.chat.id, r).await?;
        }
        ParseResult::Usage(u) => {
            bot.send_message(msg.chat.id, u).await?;
        }
    }

    Ok(())
}
