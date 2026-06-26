//! 消息生命周期：placeholder → 并发执行 → 缓存 → 原地 edit
//!
//! 命令无关：由 `ParseResult` + `shared::Cmd` 驱动，不引用任何具体命令。

use std::time::Instant;

use futures::future::join_all;
use teloxide::prelude::*;
use teloxide::types::{
    CallbackQuery, InputFile, InputMedia, InputMediaPhoto, ParseMode, ReplyParameters,
};
use tracing::error;

use crate::agent::AgentClient;
use crate::cache::{self, Cache, CacheEntry, NodeResult, gen_id};
use crate::commands::escape_markdown_v2;
use crate::error::ResponseResult;
use crate::message::{build_keyboard, format_result, msg_ids, parse_callback_data};

pub enum ReplyType {
    Text(String),
    /// 闭包在 `spawn_blocking` 上执行，避免阻塞 dispatcher
    Image(Box<dyn FnOnce() -> anyhow::Result<Vec<u8>> + Send>),
}

/// 执行命令流程
///
/// 1. 发送 placeholder
/// 2. 在所有节点并发执行
/// 3. 缓存结果
/// 4. 原地 edit placeholder 为首节点结果 + 节点切换 keyboard
///
/// parse_mode: MarkdownV2
pub async fn run_cmd_agents(
    bot: &Bot,
    msg: &Message,
    agent: &AgentClient,
    cache: &Cache,
    placeholder: String,
    target: String,
    cmd: shared::Cmd,
) -> ResponseResult<()> {
    let nodes: Vec<String> = agent.nodes().iter().map(|n| n.name.clone()).collect();
    if nodes.is_empty() {
        bot.send_message(msg.chat.id, "No nodes configured").await?;
        return Ok(());
    }

    // 1. placeholder，引用用户消息（非删除重发）
    let placeholder_msg = bot
        .send_message(msg.chat.id, placeholder)
        .reply_parameters(ReplyParameters::new(msg.id))
        .await?;

    // 2. 全节点并发执行
    let futs = nodes.iter().map(|name| {
        let name = name.clone();
        let cmd = cmd.clone();
        async move {
            let output = agent.execute(&name, cmd).await;
            NodeResult { node: name, output }
        }
    });
    let results: Vec<NodeResult> = join_all(futs).await;

    // 3. 缓存，cache_id 存入 entry；cmd_name 取自 Cmd::name()
    let cache_id = gen_id();
    let entry = CacheEntry {
        cache_id: cache_id.clone(),
        cmd_name: cmd.name().to_string(),
        target,
        nodes: nodes.clone(),
        results: results.clone(),
        created_at: Instant::now(),
    };
    {
        let mut map = cache.lock().await;
        map.insert(cache_id.clone(), entry.clone());
        cache::sweep_if_needed(&mut map);
    }

    // 4. 原地 edit placeholder：首节点结果 + keyboard
    let text = format_result(&results[0]);
    let keyboard = build_keyboard(&entry, 0);
    bot.edit_message_text(msg.chat.id, placeholder_msg.id, text)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

/// 执行命令流程（will not dispatch to agent）
/// 1. 发送 placeholder
/// 2. 原地 edit placeholder 为结果
///
/// type: PlainText/Image
pub async fn run_cmd(
    bot: &Bot,
    msg: &Message,
    placeholder: Option<String>,
    data: ReplyType,
) -> ResponseResult<()> {
    match placeholder {
        Some(ph) => {
            // 1. placeholder，引用用户消息
            let placeholder_msg = bot
                .send_message(msg.chat.id, ph)
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;

            // 2. 原地 edit placeholder：结果
            match data {
                ReplyType::Text(text) => {
                    let text = escape_markdown_v2(text);
                    bot.edit_message_text(msg.chat.id, placeholder_msg.id, text)
                        .parse_mode(ParseMode::MarkdownV2)
                        .await?;
                }
                ReplyType::Image(capture) => {
                    // 阻塞线程池，placeholder 已发出，dispatcher 不受阻塞
                    match tokio::task::spawn_blocking(capture).await {
                        Ok(Ok(img)) => {
                            bot.edit_message_media(
                                msg.chat.id,
                                placeholder_msg.id,
                                InputMedia::Photo(InputMediaPhoto::new(InputFile::memory(img))),
                            )
                            .await?;
                        }
                        Ok(Err(e)) => {
                            let err = format!("截图失败: {e}");
                            error!("{}", &err);
                            bot.edit_message_text(msg.chat.id, placeholder_msg.id, err)
                                .await?;
                        }
                        Err(e) => {
                            let err = format!("截图任务异常: {e}");
                            error!("{}", &err);
                            bot.edit_message_text(msg.chat.id, placeholder_msg.id, err)
                                .await?;
                        }
                    }
                }
            }
        }
        None => {
            // No placeholder, send result directly
            match data {
                ReplyType::Text(text) => {
                    let text = escape_markdown_v2(text);
                    bot.send_message(msg.chat.id, text)
                        .parse_mode(ParseMode::MarkdownV2)
                        .await?;
                }
                ReplyType::Image(capture) => {
                    // 阻塞线程池，placeholder 已发出，dispatcher 不受阻塞
                    match tokio::task::spawn_blocking(capture).await {
                        Ok(Ok(img)) => {
                            bot.send_photo(msg.chat.id, InputFile::memory(img)).await?;
                        }
                        Ok(Err(e)) => {
                            let err = format!("截图失败: {e}");
                            error!("{}", &err);
                            bot.send_message(msg.chat.id, err).await?;
                        }
                        Err(e) => {
                            let err = format!("截图任务异常: {e}");
                            error!("{}", &err);
                            bot.send_message(msg.chat.id, err).await?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// 处理 inline keyboard 节点切换回调
pub async fn handle_callback(bot: Bot, q: CallbackQuery, cache: Cache) -> ResponseResult<()> {
    let Some(data) = q.data.as_deref() else {
        return Ok(());
    };
    let Some((cache_id, idx)) = parse_callback_data(data) else {
        return Ok(());
    };

    let entry = {
        let map = cache.lock().await;
        map.get(cache_id).cloned()
    };
    let entry = match entry {
        Some(e) => e,
        None => {
            if let Some(m) = q.message.as_ref() {
                let (chat_id, msg_id) = msg_ids(m);
                bot.edit_message_text(chat_id, msg_id, "Result expired, please run again.")
                    .await?;
            }
            bot.answer_callback_query(q.id).await?;
            return Ok(());
        }
    };

    let idx = if idx < entry.nodes.len() { idx } else { 0 };

    if let Some(m) = q.message.as_ref() {
        let (chat_id, msg_id) = msg_ids(m);
        let keyboard = build_keyboard(&entry, idx);
        bot.edit_message_text(chat_id, msg_id, format_result(&entry.results[idx]))
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(keyboard)
            .await?;
    }

    bot.answer_callback_query(q.id).await?;
    Ok(())
}
