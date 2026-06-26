//! Extension CMD: /peer
//! 
//! 返回 Peering 信息，不调用 agent。

use crate::commands::{ParseResult, TgCommand};

pub struct Peer;

impl TgCommand for Peer {
    fn parse(&self, _text: &str) -> ParseResult {
        ParseResult::Reply {
            text: "🔗 Peering\n\
        This bot is a network tools bot and does not handle peering requests.\n\
        Please use the WebUI to manage your peers.\n\
        WebUI testing: https://peer.vcox.tech/
        "
            .into(),
            placeholder: None,
        }
    }
}
