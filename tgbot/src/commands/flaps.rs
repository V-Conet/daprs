//! /flaps 命令

use std::ffi::OsStr;

use headless_chrome::{
    Browser, LaunchOptions, protocol::cdp::Page::CaptureScreenshotFormatOption::Png, types::Bounds,
};

use crate::{
    commands::{ParseResult, TgCommand},
    config,
};

pub struct Flaps;

impl TgCommand for Flaps {
    fn parse(&self, _text: &str) -> super::ParseResult {
        ParseResult::ReplyImage {
            data: get_reply().unwrap(),
            placeholder: Some("⏳ Fetching FlapAlerted info...".into()),
        }
    }
}

fn get_reply() -> Result<Vec<u8>, anyhow::Error> {
    let launch = LaunchOptions::default_builder()
        .window_size(Some((1280, 720)))
        .args(vec![
            OsStr::new("--hide-scrollbars"),
            OsStr::new("--force-device-scale-factor=2"),
        ])
        .build()?;

    let browser = Browser::new(launch)?;

    let tab = browser.new_tab()?;

    match config::config()
        .lock()
        .unwrap()
        .tgbot
        .settings
        .flap_url
        .clone()
    {
        Some(url) => {
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
            })
            .unwrap();

            // from_surface = true; viewport now equals the full page.
            Ok(tab.capture_screenshot(Png, None, None, true)?)
        }
        None => {
            return Err(anyhow::anyhow!("No flaps server configured"));
        }
    }
}
