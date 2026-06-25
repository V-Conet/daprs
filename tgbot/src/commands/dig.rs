//! /dig 命令
//!
//! 用法：/dig <domain> [@server] [type]

use crate::commands::{ParseResult, TgCommand};

pub struct Dig;

impl TgCommand for Dig {
    fn parse(&self, text: &str) -> ParseResult {
        let mut parts = text.split_whitespace().skip(1);

        let target = match parts.next() {
            Some(t) => t.to_string(),
            None => {
                return ParseResult::Usage("Usage: /dig <domain> [@server] [type]".into());
            }
        };

        let mut server: Option<String> = None;
        let mut qtype_str = "A";
        for arg in parts {
            if let Some(stripped) = arg.strip_prefix('@') {
                server = Some(stripped.to_string());
            } else {
                qtype_str = arg;
            }
        }

        let qtype = match qtype_str.to_ascii_uppercase().as_str() {
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
        let placeholder =
            format!("⏳ DNS lookup for {target} (type: {qtype_str}, server: {server_info})...");

        ParseResult::Run {
            target: target.clone(),
            placeholder,
            cmd: shared::Cmd::Dig {
                target,
                qtype,
                server,
            },
        }
    }
}
