//! /ping 命令

use crate::commands::{MsgType, TgCommand};

pub struct Ping;

impl TgCommand for Ping {
    fn parse(&self, text: &str) -> MsgType {
        let target = match text.split_whitespace().nth(1) {
            Some(t) => t.to_string(),
            None => return MsgType::Usage("Usage: /ping <target>".into()),
        };
        let placeholder = format!("⏳ Pinging {target}...");
        MsgType::Run {
            target: target.clone(),
            placeholder,
            cmd: shared::Cmd::Ping {
                target,
                protocol: None,
                count: Some(4),
                size: None,
                dfrag: None,
                timeout: None,
            },
        }
    }
}
