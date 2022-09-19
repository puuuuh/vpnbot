use teloxide::{
    adaptors::DefaultParseMode,
    filter_command,
    prelude::*,
    utils::{command::BotCommands, markdown::escape},
};

use std::{error::Error, sync::Arc};

use crate::{
    service::{ClientInfo, ServerInfo, Service, ServiceError},
    traits::TelegramDb,
    utils,
};

#[derive(BotCommands, Clone)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "register using your key.")]
    RequestWithKey(String),
    #[command(description = "request config.")]
    Request,
    #[command(description = "help.")]
    Help,
}

#[derive(BotCommands, Clone)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum AdminCommand {
    #[command(description = "register using your key.")]
    RegisterWithKey(String),
    #[command(description = "register config.")]
    Register,
    #[command(description = "approve request.")]
    Approve,
    #[command(description = "decline request.")]
    Decline,
    #[command(description = "add admin.")]
    AddAdmin(i64),
    #[command(description = "remove admin.")]
    RmAdmin(i64),
    #[command(description = "help.")]
    Help,
}

enum Answer<'a> {
    Success,
    Config(ClientInfo, &'a ServerInfo),
    Error(String),
}

impl<'a> From<Result<(ClientInfo, &'a ServerInfo), ServiceError>> for Answer<'a> {
    fn from(r: Result<(ClientInfo, &'a ServerInfo), ServiceError>) -> Self {
        match r {
            Ok((client, server)) => Self::Config(client, server),
            Err(e) => Self::Error(e.to_string()),
        }
    }
}

impl From<Result<(), Box<dyn std::error::Error + Send + Sync>>> for Answer<'static> {
    fn from(
        r: std::result::Result<
            (),
            std::boxed::Box<(dyn std::error::Error + std::marker::Send + std::marker::Sync)>,
        >,
    ) -> Self {
        match r {
            Ok(_) => Self::Success,
            Err(e) => Self::Error(e.to_string()),
        }
    }
}

impl<'a> Answer<'a> {
    fn to_msg(&self) -> String {
        match self {
            Answer::Config(c, s) => {
                format!(
                    "Your config:\n ```\n{conf}\n```",
                    conf = escape(&utils::format_config(c, s))
                )
            }
            Answer::Error(e) => format!("Error: {e}", e = escape(&e.to_string())),
            Answer::Success => "Success\\!".to_owned(),
        }
    }
}

async fn admin<T: TelegramDb + 'static>(
    bot: DefaultParseMode<AutoSend<Bot>>,
    message: Message,
    command: AdminCommand,
    service: Arc<Service>,
    database: Arc<T>,
    server_info: Arc<ServerInfo>,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    match command {
        AdminCommand::RegisterWithKey(key) => {
            let answer: Answer = service
                .new_client(Some(key))
                .await
                .map(|res| (res, &*server_info))
                .into();

            bot.send_message(message.chat.id, answer.to_msg()).await?
        }
        AdminCommand::Register => {
            let answer: Answer = service
                .new_client(None)
                .await
                .map(|res| (res, &*server_info))
                .into();

            bot.send_message(message.chat.id, answer.to_msg()).await?
        }
        AdminCommand::Approve => todo!(),
        AdminCommand::Decline => todo!(),
        AdminCommand::AddAdmin(uid) => {
            let answer: Answer = database.add_admin(uid).await.into();

            bot.send_message(message.chat.id, answer.to_msg()).await?
        }
        AdminCommand::RmAdmin(uid) => {
            let answer: Answer = database.rm_admin(uid).await.into();

            bot.send_message(message.chat.id, answer.to_msg()).await?
        }
        AdminCommand::Help => {
            bot.send_message(
                message.chat.id,
                escape(&AdminCommand::descriptions().to_string()),
            )
            .await?
        }
    };

    Ok(())
}

async fn user<T: TelegramDb + 'static>(
    bot: DefaultParseMode<AutoSend<Bot>>,
    message: Message,
    command: Command,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    match command {
        Command::Request => {
            dbg!("testcode");
        }
        Command::RequestWithKey(_) => todo!(),
        Command::Help => {
            bot.send_message(
                message.chat.id,
                escape(&Command::descriptions().to_string()),
            )
            .await?;
        }
    };

    Ok(())
}

async fn is_admin<T: TelegramDb + 'static>(msg: Message, database: Arc<T>) -> bool {
    matches!(database.is_admin(msg.chat.id.0).await, Ok(true))
}

pub async fn start<T: TelegramDb + 'static>(
    service: Arc<Service>,
    db: Arc<T>,
    admin_uid: i64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    db.add_admin(admin_uid).await?;
    let server_info = service.server_info().await?;
    println!("start");
    tracing::info!("Starting command bot...");

    let bot = Bot::from_env()
        .auto_send()
        .parse_mode(teloxide::types::ParseMode::MarkdownV2);

    let ignore_update = |_upd| Box::pin(async {});

    Dispatcher::builder(
        bot,
        Update::filter_message()
            .branch(
                filter_command::<AdminCommand, _>()
                    .filter_async(is_admin::<T>)
                    .chain(dptree::endpoint(admin::<T>)),
            )
            .branch(filter_command::<Command, _>().chain(dptree::endpoint(user::<T>))),
    )
    .dependencies(dptree::deps![service, db, Arc::new(server_info)])
    .default_handler(ignore_update)
    .build()
    .dispatch()
    .await;
    Ok(())
}