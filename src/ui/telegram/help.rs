use std::error::Error;

use teloxide::{
    adaptors::DefaultParseMode,
    dispatching::{DpHandlerDescription, HandlerExt, UpdateFilterExt},
    prelude::{DependencyMap, Endpoint},
    requests::Requester,
    types::{Message, Update},
    utils::{command::BotCommands, markdown::escape},
    Bot,
};

use crate::traits::TelegramDb;

use super::{client, user};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Help:")]
enum Command {
    #[command(description = "help")]
    Help,
}

async fn handler<T: TelegramDb + 'static>(
    bot: DefaultParseMode<Bot>,
    message: Message,
    command: Command,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    match command {
        Command::Help => {
            let answer: String = escape(&format!(
                "{help}\n\n{user}\n\n{client}",
                help = Command::descriptions(),
                user = user::Command::descriptions(),
                client = client::Command::descriptions(),
            ));

            bot.send_message(message.chat.id, answer).await?;
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
