use teloxide::prelude::*;
use teloxide::{
    Bot,
    requests::{Requester, ResponseResult},
    types::Message,
    utils::command::BotCommands,
};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    /// Display this text.
    #[command(alias = "start")]
    Help,
    Ping,
    Dig,
    Whois,
    Trace,
    Route,
    Path,
    #[command(aliases = ["tcping4", "tcping6"] )]
    TcPing,
}

pub async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    use Command::*;
    match cmd {
        Help => help(bot, msg).await,
        _ => Ok(()),
    }
}

async fn help(bot: Bot, msg: Message) -> ResponseResult<()> {
    let text = r#"
    # Supported Commands:
    - /help: Display this help message.
    - /ping <host>: Ping a host.
    - /dig <domain>: Perform a DNS lookup for a domain.
    - /whois <domain>: Get WHOIS information for a domain.
    - /trace <host>: Perform a traceroute to a host.
    - /route <host>: Get routing information for a host.
    - /path <host>: Get the network path to a host.
    - /tcping <host>: Perform a TCP ping to a host.
    "#
    .to_string();
    bot.send_message(msg.chat.id, text).await?;
    // bot.send_message(msg.chat.id, Command::descriptions()).await?;
    Ok(())
}
