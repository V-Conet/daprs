//! /start 命令
//!
//! 静态欢迎信息，不调用 agent。

use crate::commands::{ParseResult, TgCommand};

pub struct Start;

impl TgCommand for Start {
    fn parse(&self, _text: &str) -> ParseResult {
        ParseResult::Reply(
            "🤖 DN42 Network Tools Bot\n\
             \n\
             Welcome! This bot provides network diagnostic tools.\n\
             Use /help to see available commands.\n\n\
             你好！这是一个网络诊断工具机器人。\n\
             使用 /help 查看可用命令。\n\n\
             Channel: @as423322\nDM: @v_conet"
                .into(),
        )
    }
}
