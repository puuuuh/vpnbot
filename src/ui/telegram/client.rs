use std::{error::Error, net::Ipv4Addr, str::FromStr, sync::Arc};

use teloxide::{
    adaptors::DefaultParseMode,
    dispatching::{DpHandlerDescription, HandlerExt, UpdateFilterExt},
    prelude::{DependencyMap, Endpoint},
    requests::Requester,
    types::{Message, Update},
    utils::command::BotCommands,
    Bot,
};

use crate::{
    service::{Association, Wgcfg},
    traits::TelegramDb,
};

use super::Answer;

#[derive(Debug, Clone, Copy)]
pub enum VpnMode {
    Double,
    Single,
}

impl FromStr for VpnMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "double" => Ok(Self::Double),
            "single" => Ok(Self::Single),
            _unsupported => Err("unsupported".to_string()),
        }
    }
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Peer management:")]
pub enum Command {
    #[command(description = "rename peer", parse_with = "split")]
    Rename(Ipv4Addr, String),
    #[command(
        description = "change vpn mode (supported: double, single)",
        parse_with = "split"
    )]
    VpnMode(Ipv4Addr, VpnMode),
}

async fn handler<T: TelegramDb + 'static>(
    bot: DefaultParseMode<Bot>,
    message: Message,
    command: Command,
    service: Arc<Wgcfg>,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    match command {
        Command::Rename(ip, name) => {
            //let answer: Answer = service.rename_client(ip, name).await.into();

            //bot.send_message(message.chat.id, answer.to_msg()).await?;
        }
        Command::VpnMode(ip, mode) => {
            let double = match mode {
                VpnMode::Double => true,
                VpnMode::Single => false,
            };
            let answer: Answer = service.change_settings(ip, double).await.into();
            bot.send_message(message.chat.id, answer.to_msg()).await?;
        }
    };

    Ok(())
}

async fn is_associated<T: TelegramDb + 'static>(
    bot: DefaultParseMode<Bot>,
    msg: Message,
    cmd: Command,
    service: Arc<Wgcfg>,
) -> bool {
    let ip = match cmd {
        Command::Rename(ip, _) => ip,
        Command::VpnMode(ip, _) => ip,
    };
    let access = matches!(
        service
            .association_exists(ip, Association::Telegram(msg.chat.id.0))
            .await,
        Ok(true)
    );
    if !access {
        let _ = bot.send_message(msg.chat.id, "Access denied").await;
    }
    access
}

pub fn entry<T: TelegramDb + 'static>() -> Endpoint<
    'static,
    DependencyMap,
    Result<(), Box<dyn Error + Send + Sync + 'static>>,
    DpHandlerDescription,
> {
    Update::filter_message()
        .filter_command::<Command>()
        .filter_async(is_associated::<T>)
        .endpoint(handler::<T>)
}
