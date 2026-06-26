//! /path 命令

use crate::commands::{MsgType, TgCommand};

pub struct Path;

impl TgCommand for Path {
    fn parse(&self, text: &str) -> MsgType {
        let target = match text.split_whitespace().nth(1) {
            Some(t) => t.to_string(),
            None => return MsgType::Usage("Usage: /path <target>".into()),
        };
        let placeholder = format!("⏳ Looking up AS path to {target}...");
        MsgType::Run {
            target: target.clone(),
            placeholder,
            cmd: shared::Cmd::Path {
                target,
                protocol: None,
            },
        }
    }
}
