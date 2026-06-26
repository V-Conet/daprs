//! /start 命令
//!
//! 静态欢迎信息，不调用 agent。

use crate::{
    commands::{MsgType, TgCommand},
    config,
};

pub struct Start;

impl TgCommand for Start {
    fn parse(&self, _text: &str) -> MsgType {
        // 读取配置文件中的 start_msg
        MsgType::Reply {
            text: (config::config()
                .lock()
                .unwrap()
                .tgbot
                .settings
                .start_msg
                .clone()),
            placeholder: None,
        }
    }
}
