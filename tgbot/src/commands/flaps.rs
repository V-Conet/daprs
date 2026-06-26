//! /flaps 命令
//!
//! 截取 flaps 页面整页截图并回复为图片。截图是重活儿（启动 Chrome +
//! 导航 + 渲染），故以闭包形式交给 `flow::run_cmd`，由其在 `spawn_blocking`
//! 上执行，避免阻塞 teloxide dispatcher。

use std::ffi::OsStr;

use anyhow::anyhow;
use headless_chrome::{
    Browser, LaunchOptions, protocol::cdp::Page::CaptureScreenshotFormatOption, types::Bounds,
};
use tracing::warn;

use crate::{
    commands::{MsgType, TgCommand},
    config,
};

pub struct Flaps;

impl TgCommand for Flaps {
    fn parse(&self, _text: &str) -> MsgType {
        MsgType::ReplyImage {
            placeholder: Some("⏳ Fetching FlapAlerted...".into()),
            capture: Box::new(capture_png),
        }
    }
}

/// Capture a screenshot on a separate thread
fn capture_png() -> anyhow::Result<Vec<u8>> {
    let url = config::config()
        .lock()
        .unwrap()
        .tgbot
        .settings
        .flap_url
        .clone()
        .ok_or_else(|| anyhow!("No flaps server configured"))?;

    let launch = LaunchOptions::default_builder()
        .window_size(Some((1280, 720)))
        .sandbox(false)
        .args(vec![
            OsStr::new("--hide-scrollbars"),
            OsStr::new("--force-device-scale-factor=2"),
        ])
        .build()?;

    let browser = Browser::new(launch)?;
    let tab = browser.new_tab()?;

    tab.navigate_to(&url)?;
    tab.wait_until_navigated()?;
    tab.wait_for_element("#loadingScreen[style*='display: none']")?;

    let h = tab
        .evaluate("document.documentElement.scrollHeight", false)?
        .value
        .and_then(|v| v.as_f64())
        .unwrap();
    let w = tab
        .evaluate("document.documentElement.scrollWidth", false)?
        .value
        .and_then(|v| v.as_f64())
        .unwrap();

    tab.set_bounds(Bounds::Normal {
        left: Some(0),
        top: Some(0),
        width: Some(w),
        height: Some(h),
    })?;

    // from_surface = true; viewport now equals the full page.
    Ok(tab.capture_screenshot(CaptureScreenshotFormatOption::Png, None, None, true)?)
}
