//! /tcping 命令
//!
//! 用法：/tcping <host> <port>

use crate::commands::{MsgType, TgCommand};

pub struct TcPing;

impl TgCommand for TcPing {
    fn parse(&self, text: &str) -> MsgType {
        let mut parts = text.split_whitespace().skip(1);

        let target = match parts.next() {
            Some(t) => t.to_string(),
            None => return MsgType::Usage("Usage: /tcping <host> <port>".into()),
        };
        let port: u16 = match parts.next() {
            Some(p) => match p.parse() {
                Ok(v) => v,
                Err(_) => return MsgType::Usage("Port must be a number".into()),
            },
            None => return MsgType::Usage("Usage: /tcping <host> <port>".into()),
        };

        let placeholder = format!("⏳ TCP ping to {target}:{port}...");
        MsgType::Run {
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
