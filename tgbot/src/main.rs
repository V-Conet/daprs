use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use teloxide::{
    dispatching::UpdateFilterExt,
    dptree,
    prelude::*,
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, ParseMode, Update},
};
use tokio::sync::Mutex;

use crate::cmd::Command;
use crate::config::Config;

mod agent;
mod cmd;
mod config;

type Cache = Arc<Mutex<HashMap<String, CacheEntry>>>;

#[derive(Clone)]
pub struct CacheEntry {
    pub results: Vec<NodeResult>,
    pub nodes: Vec<String>,
    pub cmd_name: String,
    pub target: String,
}

#[derive(Clone)]
pub struct NodeResult {
    pub node: String,
    pub output: Result<String, String>,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = config::Cli::parse();
    let config = load_config(&cli.config)?;
    let config = Arc::new(config);

    tracing_subscriber::fmt::init();

    let bot = Bot::from_env();
    let cache: Cache = Arc::new(Mutex::new(HashMap::new()));

    let command_handler =
        Update::filter_message()
            .filter_command::<Command>()
            .branch(dptree::endpoint({
                let cache = cache.clone();
                let config = config.clone();
                move |msg: Message, cmd: Command, bot: Bot| {
                    let cache = cache.clone();
                    let config = config.clone();
                    async move {
                        cmd::answer(bot, msg, cmd, (*config).clone(), cache).await?;
                        Ok::<_, teloxide::RequestError>(())
                    }
                }
            }));

    let callback_handler = Update::filter_callback_query().branch(dptree::endpoint({
        let cache = cache.clone();
        move |bot: Bot, q: CallbackQuery| {
            let cache = cache.clone();
            async move {
                handle_callback(bot, q, cache).await?;
                Ok::<_, teloxide::RequestError>(())
            }
        }
    }));

    Dispatcher::builder(
        bot,
        dptree::entry()
            .branch(command_handler)
            .branch(callback_handler),
    )
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;

    Ok(())
}

async fn handle_callback(
    bot: Bot,
    q: CallbackQuery,
    cache: Cache,
) -> Result<(), teloxide::RequestError> {
    let data = match q.data {
        Some(d) => d,
        None => return Ok(()),
    };

    // format: "show_{cache_id}_{node_index}"
    let parts: Vec<&str> = data.splitn(3, '_').collect();
    if parts.len() != 3 || parts[0] != "show" {
        return Ok(());
    }

    let cache_id = parts[1];
    let idx: usize = match parts[2].parse() {
        Ok(i) => i,
        Err(_) => return Ok(()),
    };

    let entry = {
        let map = cache.lock().await;
        map.get(cache_id).cloned()
    };

    let entry = match entry {
        Some(e) => e,
        None => {
            if let Some(msg) = q.message {
                let (chat_id, msg_id) = get_message_ids(&msg);
                bot.edit_message_text(chat_id, msg_id, "Result expired, please run again.")
                    .await?;
            }
            bot.answer_callback_query(q.id).await?;
            return Ok(());
        }
    };

    let idx = if idx < entry.nodes.len() { idx } else { 0 };
    let text = format_result(&entry, idx);

    if let Some(msg) = q.message {
        let (chat_id, msg_id) = get_message_ids(&msg);
        let keyboard = build_keyboard(&entry, idx);
        bot.edit_message_text(chat_id, msg_id, &text)
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(keyboard)
            .await?;
    }

    bot.answer_callback_query(q.id).await?;
    Ok(())
}

fn get_message_ids(
    msg: &teloxide::types::MaybeInaccessibleMessage,
) -> (teloxide::types::ChatId, teloxide::types::MessageId) {
    use teloxide::types::MaybeInaccessibleMessage;
    match msg {
        MaybeInaccessibleMessage::Regular(m) => (m.chat.id, m.id),
        MaybeInaccessibleMessage::Inaccessible(_) => {
            (teloxide::types::ChatId(0), teloxide::types::MessageId(0))
        }
    }
}

fn escape_markdown(text: &str) -> String {
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

fn format_result(entry: &CacheEntry, idx: usize) -> String {
    let nr = &entry.results[idx];
    match &nr.output {
        Ok(o) => {
            let escaped = escape_markdown(o);
            format!("```\n{escaped}\n```")
        }
        Err(e) => {
            let escaped = escape_markdown(e);
            format!("❌ {escaped}")
        }
    }
}

fn build_keyboard(entry: &CacheEntry, current: usize) -> InlineKeyboardMarkup {
    let cache_id = gen_cache_id(&entry.target);
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
            InlineKeyboardButton::callback(label, format!("show_{}_{}", cache_id, i))
        })
        .collect();

    InlineKeyboardMarkup::new(vec![buttons])
}

pub fn build_keyboard_with_id(
    cache_id: &str,
    nodes: &[String],
    current: usize,
) -> InlineKeyboardMarkup {
    let buttons: Vec<InlineKeyboardButton> = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let label = if i == current {
                format!("✅ {}", node.to_uppercase())
            } else {
                node.to_uppercase()
            };
            InlineKeyboardButton::callback(label, format!("show_{}_{}", cache_id, i))
        })
        .collect();

    InlineKeyboardMarkup::new(vec![buttons])
}

pub fn format_node_result(nr: &NodeResult) -> String {
    match &nr.output {
        Ok(o) => {
            let escaped = escape_markdown(o);
            format!("```\n{escaped}\n```")
        }
        Err(e) => {
            let escaped = escape_markdown(e);
            format!("❌ {escaped}")
        }
    }
}

pub async fn cleanup_cache(cache: &Cache) {
    let mut map = cache.lock().await;
    if map.len() > 100 {
        map.clear();
    }
}

pub fn gen_id() -> String {
    uuid::Uuid::new_v4().to_string().replace('-', "")
}

fn gen_cache_id(target: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    target.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn load_config<P: AsRef<Path>>(path: P) -> Result<Config> {
    let config_content = std::fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&config_content)
        .with_context(|| format!("Failed to load config: {}", path.as_ref().display()))?;
    Ok(config)
}
