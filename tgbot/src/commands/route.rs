//! /route 命令

use crate::commands::{ParseResult, TgCommand};

pub struct Route;

impl TgCommand for Route {
    fn parse(&self, text: &str) -> ParseResult {
        let target = match text.split_whitespace().nth(1) {
            Some(t) => t.to_string(),
            None => return ParseResult::Usage("Usage: /route <target>".into()),
        };
        let placeholder = format!("⏳ Looking up route to {target}...");
        ParseResult::Run {
            target: target.clone(),
            placeholder,
            cmd: shared::Cmd::Route {
                target,
                protocol: None,
            },
        }
    }
}
