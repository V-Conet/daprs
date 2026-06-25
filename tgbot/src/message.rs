//! 消息格式化、代码块转义、截断、inline keyboard 与 callback data

use teloxide::types::{
    ChatId, InlineKeyboardButton, InlineKeyboardMarkup, MaybeInaccessibleMessage, MessageId,
};

use crate::cache::{CacheEntry, NodeResult};

/// Telegram 单条消息文本上限
const TG_MAX: usize = 4096;

/// MarkdownV2 代码块内仅 `\` 与 `` ` `` 有特殊含义。
///
/// 旧的 escape_markdown 把 `1.2.3.4` 转义成 `1\.2\.3\.4`，损坏输出。
pub fn escape_codeblock_inner(text: &str) -> String {
    text.replace('\\', "\\\\").replace('`', "\\`")
}

/// 代码块外的完整 MarkdownV2 转义（用于错误行）
pub fn escape_inline(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('_', "\\_")
        .replace('*', "\\*")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('(', "\\(")
        .replace(')', "\\)")
        .replace('~', "\\~")
        .replace('`', "\\`")
        .replace('>', "\\>")
        .replace('#', "\\#")
        .replace('+', "\\+")
        .replace('-', "\\-")
        .replace('=', "\\=")
        .replace('|', "\\|")
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('.', "\\.")
        .replace('!', "\\!")
}

/// 截断至 Telegram 4096 字符上限，预留代码块 fence 与截断提示的空间
pub fn truncate_for_message(text: &str) -> String {
    const NOTICE: &str = "\n…(truncated)";
    let budget = TG_MAX.saturating_sub("```\n".len() + "\n```".len() + NOTICE.len());
    if text.len() <= budget {
        return text.to_string();
    }
    // 按 char boundary 回退
    let mut end = budget;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{NOTICE}", &text[..end])
}

/// 格式化单个节点结果为消息文本
pub fn format_result(nr: &NodeResult) -> String {
    match &nr.output {
        Ok(o) => {
            let body = escape_codeblock_inner(&truncate_for_message(o));
            format!("```\n{body}\n```")
        }
        Err(e) => format!("❌ {}", escape_inline(&truncate_for_message(e))),
    }
}

/// 构造 callback data：`show_{cache_id}_{idx}`
pub fn callback_data(cache_id: &str, idx: usize) -> String {
    format!("show_{cache_id}_{idx}")
}

/// 解析 callback data，返回 (cache_id, idx)
pub fn parse_callback_data(data: &str) -> Option<(&str, usize)> {
    let parts: Vec<&str> = data.splitn(3, '_').collect();
    if parts.len() != 3 || parts[0] != "show" {
        return None;
    }
    let idx = parts[2].parse().ok()?;
    Some((parts[1], idx))
}

/// 构建节点切换 inline keyboard，高亮当前节点
pub fn build_keyboard(entry: &CacheEntry, current: usize) -> InlineKeyboardMarkup {
    let buttons: Vec<InlineKeyboardButton> = entry
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let label = if i == current {
                format!("✅ {}", node.to_uppercase())
            } else {
                node.to_uppercase()
            };
            InlineKeyboardButton::callback(label, callback_data(&entry.cache_id, i))
        })
        .collect();
    InlineKeyboardMarkup::new(vec![buttons])
}

/// 从 MaybeInaccessibleMessage 提取 (chat_id, message_id)
pub fn msg_ids(m: &MaybeInaccessibleMessage) -> (ChatId, MessageId) {
    match m {
        MaybeInaccessibleMessage::Regular(r) => (r.chat.id, r.id),
        MaybeInaccessibleMessage::Inaccessible(_) => (ChatId(0), MessageId(0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codeblock_inner_keeps_dots() {
        assert_eq!(escape_codeblock_inner("1.2.3.4"), "1.2.3.4");
        assert_eq!(escape_codeblock_inner("a`b\\c"), "a\\`b\\\\c");
    }

    #[test]
    fn callback_roundtrip() {
        assert_eq!(
            parse_callback_data(&callback_data("abc", 2)),
            Some(("abc", 2))
        );
        assert_eq!(parse_callback_data("nope_1_2"), None);
        assert_eq!(parse_callback_data("show_abc_x"), None);
    }

    #[test]
    fn truncate_keeps_short() {
        assert_eq!(truncate_for_message("short"), "short");
    }

    #[test]
    fn truncate_long() {
        let s = "x".repeat(5000);
        let t = truncate_for_message(&s);
        assert!(t.len() <= TG_MAX);
        assert!(t.ends_with("…(truncated)"));
    }
}
