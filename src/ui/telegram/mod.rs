mod admin;
mod client;
mod help;
mod user;

use clap::Parser;
use teloxide::{
    dispatching::dialogue::InMemStorage, prelude::*, types::UpdateKind, utils::markdown::escape,
};

use std::{error::Error, fmt::Write, sync::Arc};

use crate::{
    service::{ClientInfo, ConfigInfo, PeerInfo, Request, ServerInfo, ServiceError, User, Wgcfg},
    traits::TelegramDb,
    utils,
};

pub enum Answer<'a> {
    Success,
    Config(ConfigInfo, &'a ServerInfo),
    PairList(Vec<ClientInfo>),
    Requests(Vec<Request>),
    Peers(Vec<PeerInfo>),
    Error(String),
}

impl<'a> From<Result<(ConfigInfo, &'a ServerInfo), ServiceError>> for Answer<'a> {
    fn from(r: Result<(ConfigInfo, &'a ServerInfo), ServiceError>) -> Self {
        match r {
            Ok((client, server)) => Self::Config(client, server),
            Err(e) => Self::Error(e.to_string()),
        }
    }
}

impl From<Result<Vec<PeerInfo>, ServiceError>> for Answer<'static> {
    fn from(r: Result<Vec<PeerInfo>, ServiceError>) -> Self {
        match r {
            Ok(i) => Self::Peers(i),
            Err(e) => Self::Error(e.to_string()),
        }
    }
}

impl From<Result<Vec<ClientInfo>, ServiceError>> for Answer<'static> {
    fn from(r: Result<Vec<ClientInfo>, ServiceError>) -> Self {
        match r {
            Ok(i) => Self::PairList(i),
            Err(e) => Self::Error(e.to_string()),
        }
    }
}

impl From<Result<Vec<Request>, ServiceError>> for Answer<'static> {
    fn from(r: Result<Vec<Request>, ServiceError>) -> Self {
        match r {
            Ok(i) => Self::Requests(i),
            Err(e) => Self::Error(e.to_string()),
        }
    }
}

impl From<Result<(), ServiceError>> for Answer<'static> {
    fn from(r: Result<(), ServiceError>) -> Self {
        match r {
            Ok(_) => Self::Success,
            Err(e) => Self::Error(e.to_string()),
        }
    }
}

impl From<Result<(), Box<dyn Error + Send + Sync>>> for Answer<'static> {
    fn from(r: Result<(), Box<(dyn Error + Send + Sync)>>) -> Self {
        match r {
            Ok(_) => Self::Success,
            Err(e) => Self::Error(e.to_string()),
        }
    }
}

impl<'a> Answer<'a> {
    pub fn to_msg(&self) -> String {
        match self {
            Answer::Config(c, s) => {
                format!(
                    "Your config:\n ```\n{conf}\n```",
                    conf = escape(&utils::format_config(c, s))
                )
            }
            Answer::Error(e) => format!("Error: {e}", e = escape(&e.to_string())),
            Answer::Success => "Success\\!".to_owned(),
            Answer::PairList(clients) => {
                let mut res = "Paired ips: \n".to_string();
                for client in clients {
                    let _ = writeln!(
                        res,
                        "\t{ip} \\- {name}",
                        ip = escape(&client.ip.to_string()),
                        name = escape(client.name.as_deref().unwrap_or("<unnamed>"))
                    );
                }
                res
            }
            Answer::Requests(requests) => {
                let mut res = "Requests: \n".to_string();
                for request in requests {
                    let _ = writeln!(
                        res,
                        "\t```{id}``` \\([author](tg://user?id={uid})\\) \\- {status}",
                        id = escape(&request.id.to_string()),
                        status = escape(&request.status.to_string()),
                        uid = request.telegram_id.unwrap_or_default()
                    );
                }
                res
            }
            Answer::Peers(s) => {
                let mut msg = "Peers:\n```\n".to_string();
                let fmt_bytes = |data: u64| {
                    escape(&format!(
                        "{data:.3}",
                        data = (data as f64 / 1024.0 / 1024.0 / 1024.0)
                    ))
                };
                for p in s {
                    let _ = writeln!(
                        msg,
                        "\t↑{tx} GB, ↓{rx} GB",
                        tx = fmt_bytes(p.tx),
                        rx = fmt_bytes(p.rx),
                    );
                }
                msg.push_str("```");
                msg
            }
        }
    }
}

async fn register_user<T: TelegramDb + 'static>(upd: Update, database: Arc<T>) -> bool {
    if let Some(chat) = upd.chat() {
        let _ = database.add_user(chat.id.0).await;
    };

    true
}

async fn get_user(upd: Update, service: Arc<Wgcfg>) -> Option<User> {
    if let Some(chat) = upd.chat() {
        if let Ok(t) = service
            .user(crate::service::Association::Telegram(chat.id.0))
            .await
        {
            return Some(t);
        }
    };
    None
}

#[derive(Debug, Parser)]
pub struct Config {
    #[clap(long, short, env = "TELEGRAM_ADMIN", value_parser)]
    admin_uid: i64,
    #[clap(long, short, env = "TELEGRAM_TOKEN", value_parser)]
    token: String,
}

pub async fn start<T: TelegramDb + 'static>(
    config: Config,
    service: Wgcfg,
    db: T,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    db.add_admin(config.admin_uid).await?;
    let server_info = service.server_info().await?;
    tracing::info!("Starting command bot...");

    let bot = Bot::new(config.token).parse_mode(teloxide::types::ParseMode::MarkdownV2);

    let b = bot.clone();
    let ignore_update = move |upd: Arc<Update>| {
        let b = b.clone();
        Box::pin(async move {
            if let UpdateKind::Message(msg) = &upd.kind {
                if Some("/start") == msg.text() {
                    let _ = b.send_message(msg.chat.id, msg.chat.id.0.to_string()).await;
                }
            }
        })
    };

    Dispatcher::builder(
        bot,
        dptree::entry()
            .chain(dptree::filter_async(register_user::<T>))
            .filter_map_async(get_user)
            .branch(admin::entry::<T>()),
    )
    .dependencies(dptree::deps![
        InMemStorage::<admin::State>::new(),
        Arc::new(service),
        Arc::new(db),
        Arc::new(server_info)
    ])
    .default_handler(ignore_update)
    .build()
    .dispatch()
    .await;
    Ok(())
}
