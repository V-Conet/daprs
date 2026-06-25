//! /tcping 命令
//!
//! 用法：/tcping <host> <port>

use crate::commands::{ParseResult, TgCommand};

pub struct TcPing;

impl TgCommand for TcPing {
    fn parse(&self, text: &str) -> ParseResult {
        let mut parts = text.split_whitespace().skip(1);

        let target = match parts.next() {
            Some(t) => t.to_string(),
            None => return ParseResult::Usage("Usage: /tcping <host> <port>".into()),
        };
        let port: u16 = match parts.next() {
            Some(p) => match p.parse() {
                Ok(v) => v,
                Err(_) => return ParseResult::Usage("Port must be a number".into()),
            },
            None => return ParseResult::Usage("Usage: /tcping <host> <port>".into()),
        };

        let placeholder = format!("⏳ TCP ping to {target}:{port}...");
        ParseResult::Run {
            target: target.clone(),
            placeholder,
            cmd: shared::Cmd::TcPing {
                target,
                port,
                count: Some(5),
                timeout: Some(3),
                protocol: None,
            },
        }
    }
}
