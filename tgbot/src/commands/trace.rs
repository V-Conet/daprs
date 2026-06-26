//! /trace 命令（别名 /traceroute）

use crate::commands::{MsgType, TgCommand};

pub struct Trace;

impl TgCommand for Trace {
    fn parse(&self, text: &str) -> MsgType {
        let target = match text.split_whitespace().nth(1) {
            Some(t) => t.to_string(),
            None => return MsgType::Usage("Usage: /trace <target>".into()),
        };
        let placeholder = format!("⏳ Traceroute to {target}...");
        MsgType::Run {
            target: target.clone(),
            placeholder,
            cmd: shared::Cmd::Traceroute {
                target,
                protocol: None,
            },
        }
    }
}
