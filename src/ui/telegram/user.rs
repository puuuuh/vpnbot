use std::{error::Error, net::Ipv4Addr, sync::Arc};

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

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "User:")]
pub enum Command {
    #[command(description = "request config using your public key")]
    RequestWithKey(String),
    #[command(description = "request config")]
    Request,
    #[command(description = "check request status")]
    MyRequests,
    #[command(description = "create pairing")]
    Pair(String),
    #[command(description = "remove pairing")]
    Unpair(Ipv4Addr),
    #[command(description = "currently paired clients")]
    Pairs,
}

async fn handler<T: TelegramDb + 'static>(
    bot: DefaultParseMode<Bot>,
    message: Message,
    command: Command,
    service: Arc<Wgcfg>,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    match command {
        Command::Request => {
            let answer: Answer = service.request_config(message.chat.id.0).await.into();
            bot.send_message(message.chat.id, answer.to_msg()).await?;
        }
        Command::RequestWithKey(_) => {}
        Command::Pair(token) => {
            let answer: Answer = service
                .create_association(token, Association::Telegram(message.chat.id.0))
                .await
                .into();

            bot.send_message(message.chat.id, answer.to_msg()).await?;
        }
        Command::Unpair(ip) => {
            let answer: Answer = service
                .remove_association(ip, Association::Telegram(message.chat.id.0))
                .await
                .into();

            bot.send_message(message.chat.id, answer.to_msg()).await?;
        }
        Command::Pairs => {
            let answer: Answer = service
                .associations(Association::Telegram(message.chat.id.0))
                .await
                .into();

            bot.send_message(message.chat.id, answer.to_msg()).await?;
        }
        Command::MyRequests => {
            let answer: Answer = service.requests_by_uid(message.chat.id.0).await.into();

            bot.send_message(message.chat.id, answer.to_msg()).await?;
        }
    };

    Ok(())
}

pub fn entry<T: TelegramDb + 'static>() -> Endpoint<
    'static,
    DependencyMap,
    Result<(), Box<dyn Error + Send + Sync + 'static>>,
    DpHandlerDescription,
> {
    Update::filter_message()
        .filter_command::<Command>()
        .endpoint(handler::<T>)
}
