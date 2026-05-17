use teloxide::Bot;
use teloxide::repls::CommandReplExt;

use anyhow::Result;

use crate::cmd::Command;
use crate::cmd::answer;

pub mod cmd;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
async fn run() -> Result<()> {
    let bot = Bot::from_env();

    Command::repl(bot, answer).await;
    Ok(())
}
