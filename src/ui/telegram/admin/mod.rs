mod buttons;

use std::{error::Error, sync::Arc};

use base64::Engine;
use serde::{Deserialize, Serialize};
use teloxide::{
    adaptors::DefaultParseMode,
    dispatching::{
        dialogue::{Dialogue, InMemStorage},
        DpHandlerDescription, HandlerExt, UpdateFilterExt,
    },
    dptree,
    payloads::SendMessageSetters,
    prelude::{DependencyMap, Endpoint},
    requests::Requester,
    types::{CallbackQuery, InlineKeyboardMarkup, InputFile, Message, Update},
    utils::markdown::escape,
    Bot,
};
use uuid::Uuid;

use crate::{
    service::{User, Wgcfg},
    traits::TelegramDb,
};

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
type MyDialogue = Dialogue<State, InMemStorage<State>>;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    MainMenu,
    ConfigsMenu,
    Config(Uuid),
    RenameConfig(Uuid),
    CreateConfig,
    Admins,
    AddAdmin,
}

#[derive(Deserialize, Serialize, Clone)]
enum Action {
    OpenMain,
    Configs,
    Config(Uuid),
    CreateConfig,
    RenameConfig(Uuid),
    RemoveConfig(Uuid),
    GetConfigFile(Uuid),
    Admins,
    AddAdmin,
    RmAdmin(Uuid),
}

impl State {
    pub async fn msg(
        &self,
        user: &User,
        service: &Wgcfg,
    ) -> Result<(String, Option<InlineKeyboardMarkup>), Box<dyn std::error::Error + Send + Sync>>
    {
        match self {
            State::Start => {
                todo!()
            }
            State::MainMenu => Ok((
                "Main menu".to_owned(),
                Some(InlineKeyboardMarkup::new([
                    [buttons::CONFIGS.clone()],
                    [buttons::ADMINS.clone()],
                ])),
            )),
            State::ConfigsMenu => {
                let configs = service.configs(user.id).await?;

                let mut rows = Vec::with_capacity(10);
                let mut chunks = configs.into_iter().map(buttons::config);
                loop {
                    match (chunks.next(), chunks.next()) {
                        (None, None) => break,
                        (None, Some(_)) => {}
                        (Some(a), None) => {
                            rows.push(vec![a]);
                        }
                        (Some(a), Some(b)) => {
                            rows.push(vec![a, b]);
                        }
                    };
                }
                rows.push(vec![buttons::CREATE_CONFIG.clone()]);
                rows.push(vec![buttons::MAIN_MENU.clone()]);

                Ok(("Configs".to_owned(), Some(InlineKeyboardMarkup::new(rows))))
            }
            State::Config(id) => {
                let c = service.config(user, *id).await?;
                let cap = format!(
                    "Name: {}\nIP: {}\nKey: {}\nTx: {:.3} GB\nRx: {:.3} GB",
                    escape(&c.config.name),
                    escape(&c.config.ip.to_string()),
                    escape(&base64::engine::general_purpose::STANDARD.encode(c.config.pub_key)),
                    escape(&(c.stats.tx as f64 / (1024u64 * 1024 * 1024) as f64).to_string()),
                    escape(&(c.stats.rx as f64 / (1024u64 * 1024 * 1024) as f64).to_string()),
                );
                Ok((
                    cap,
                    Some(InlineKeyboardMarkup::new([
                        [
                            buttons::config_rename(&c.config),
                            buttons::config_remove(&c.config),
                            buttons::config_file(&c.config),
                        ]
                        .as_slice()
                        .iter()
                        .cloned(),
                        [buttons::MAIN_MENU.clone()].as_slice().iter().cloned(),
                    ])),
                ))
            }
            State::RenameConfig(_) => Ok(("Enter new name: ".to_owned(), None)),
            State::CreateConfig => Ok(("Enter name: ".to_owned(), None)),
            State::Admins => Ok((
                "Admins:".to_string(),
                Some(InlineKeyboardMarkup::new([
                    [buttons::ADD_ADMIN.clone()],
                    [buttons::MAIN_MENU.clone()],
                ])),
            )),
            State::AddAdmin => Ok(("Enter uid: ".to_owned(), None)),
        }
    }
}

async fn is_admin(user: User) -> bool {
    user.is_admin()
}

pub fn entry<T: TelegramDb + 'static>() -> Endpoint<
    'static,
    DependencyMap,
    Result<(), Box<dyn Error + Send + Sync + 'static>>,
    DpHandlerDescription,
> {
    dptree::entry()
        .filter_async(is_admin)
        .branch(
            Update::filter_message()
                .enter_dialogue::<Update, InMemStorage<State>, State>()
                .branch(dptree::case![State::RenameConfig(config_id)].endpoint(config_rename))
                .branch(dptree::case![State::CreateConfig].endpoint(config_create))
                .branch(dptree::case![State::AddAdmin].endpoint(add_admin))
                .branch(dptree::endpoint(start)),
        )
        .branch(
            Update::filter_callback_query()
                .enter_dialogue::<Update, InMemStorage<State>, State>()
                .endpoint(callback_handler),
        )
}

async fn start(
    bot: DefaultParseMode<Bot>,
    service: Arc<Wgcfg>,
    dialogue: MyDialogue,
    msg: Message,
    user: User,
) -> HandlerResult {
    let next_state = State::MainMenu;
    if let Ok((cap, kb)) = next_state.msg(&user, &service).await {
        let mut t = bot.send_message(msg.chat.id, cap);
        if let Some(kb) = kb {
            t = t.reply_markup(kb);
        }
        t.await?;
    }
    dialogue.update(State::MainMenu).await?;

    Ok(())
}

async fn config_rename(
    bot: DefaultParseMode<Bot>,
    service: Arc<Wgcfg>,
    dialogue: MyDialogue,
    msg: Message,
    config_id: Uuid,
    user: User,
) -> HandlerResult {
    if let Some(n) = msg.text() {
        service.rename_config(&user, config_id, n).await?;
    } else {
        bot.send_message(msg.chat.id, "Unexpected msg").await?;
    }

    let next_state = State::Config(config_id);
    if let Ok((cap, kb)) = next_state.msg(&user, &service).await {
        let mut t = bot.send_message(msg.chat.id, cap);
        if let Some(kb) = kb {
            t = t.reply_markup(kb);
        }
        t.await?;
    }

    dialogue.update(next_state).await?;
    Ok(())
}

async fn config_create(
    bot: DefaultParseMode<Bot>,
    service: Arc<Wgcfg>,
    dialogue: MyDialogue,
    msg: Message,
    user: User,
) -> HandlerResult {
    let Some(n) = msg.text() else {
        bot.send_message(msg.chat.id, "Unexpected msg").await?;
        return Ok(())
    };

    let config_id = service.new_config(&user, n.to_owned(), None).await?;

    let next_state = State::Config(config_id);
    if let Ok((cap, kb)) = next_state.msg(&user, &service).await {
        let mut t = bot.send_message(msg.chat.id, cap);
        if let Some(kb) = kb {
            t = t.reply_markup(kb);
        }
        t.await?;
    }

    dialogue.update(next_state).await?;
    Ok(())
}

async fn add_admin(
    bot: DefaultParseMode<Bot>,
    service: Arc<Wgcfg>,
    dialogue: MyDialogue,
    msg: Message,
    user: User,
) -> HandlerResult {
    let Some(n) = msg.text() else {
        bot.send_message(msg.chat.id, "Unexpected msg").await?;
        return Ok(())
    };
    let Ok(n) = str::parse::<i64>(n) else {
        bot.send_message(msg.chat.id, "Invalid user id").await?;
        return Ok(())
    };
    let Ok(new_admin) = service.user(crate::service::Association::Telegram(n)).await else {
        bot.send_message(msg.chat.id, "Unknown user id").await?;
        return Ok(())
    };

    service.add_admin(&user, new_admin.id).await?;

    let next_state = State::Admins;
    if let Ok((cap, kb)) = next_state.msg(&user, &service).await {
        let mut t = bot.send_message(msg.chat.id, cap);
        if let Some(kb) = kb {
            t = t.reply_markup(kb);
        }
        t.await?;
    }

    dialogue.update(next_state).await?;
    Ok(())
}

async fn callback_handler(
    bot: DefaultParseMode<Bot>,
    dialogue: MyDialogue,
    service: Arc<Wgcfg>,
    user: User,
    q: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(action) = q.data {
        let a = serde_json::from_str(&action)?;
        bot.answer_callback_query(q.id).await?;

        if let Action::GetConfigFile(id) = a {
            let config = service.config(&user, id).await?;
            let file = config.config.config_file(service.server_info().await?);
            let mut name = config.config.name;
            name.push_str(".conf");
            bot.send_document(dialogue.chat_id(), InputFile::memory(file).file_name(name))
                .await?;
        };
        if let Action::RemoveConfig(id) = a {
            service.rm_config(&user, id).await?;
        };
        if let Action::RmAdmin(id) = a {
            service.rm_admin(&user, id).await?;
        };
        let next_state = match a {
            Action::OpenMain => State::MainMenu,
            Action::Configs => State::ConfigsMenu,
            Action::Config(id) => State::Config(id),
            Action::RenameConfig(id) => State::RenameConfig(id),
            Action::GetConfigFile(id) => State::Config(id),
            Action::CreateConfig => State::CreateConfig,
            Action::RemoveConfig(_) => State::MainMenu,
            Action::Admins => State::Admins,
            Action::AddAdmin => State::AddAdmin,
            Action::RmAdmin(_) => State::Admins,
        };

        if let Ok((cap, kb)) = next_state.msg(&user, &service).await {
            let mut t = bot.send_message(dialogue.chat_id(), cap);
            if let Some(kb) = kb {
                t = t.reply_markup(kb);
            }
            t.await?;
        }

        dialogue.update(next_state).await?;
    }

    Ok(())
}
