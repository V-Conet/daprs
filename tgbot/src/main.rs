//! DAPRS TgBot - DN42 Network Tools Telegram Bot

use std::collections::HashMap;
use std::sync::Arc;

use clap::Parser;
use teloxide::dispatching::UpdateFilterExt;
use teloxide::dptree;
use teloxide::prelude::*;
use teloxide::types::{CallbackQuery, Message, Update};
use tokio::sync::Mutex;
use tracing::info;

use crate::agent::AgentClient;
use crate::cache::Cache;
use crate::commands::{Command, Registry, dispatch};
use crate::flow::handle_callback;

mod agent;
mod cache;
mod commands;
mod config;
mod error;
mod flow;
mod message;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    let cli = config::Cli::parse();
    let config = Arc::new(config::load_config(&cli.config)?);

    config::init_config((*config).clone());

    tracing_subscriber::fmt::init();

    info!("DAPRS TgBot v{}", env!("CARGO_PKG_VERSION"));

    let bot = Bot::from_env();
    let cache: Cache = Arc::new(Mutex::new(HashMap::new()));
    let agent = Arc::new(AgentClient::new((*config).clone()));
    let registry = Arc::new(Registry::build());

    let command_handler =
        Update::filter_message()
            .filter_command::<Command>()
            .branch(dptree::endpoint({
                let agent = agent.clone();
                let registry = registry.clone();
                let cache = cache.clone();
                move |msg: Message, cmd: Command, bot: Bot| {
                    let agent = agent.clone();
                    let registry = registry.clone();
                    let cache = cache.clone();
                    async move {
                        dispatch(bot, msg, cmd, agent, cache, registry).await?;
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
