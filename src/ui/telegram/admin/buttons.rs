use std::sync::LazyLock;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardButtonKind};

use crate::service::configs::Config;

use super::Action;

pub static MAIN_MENU: LazyLock<InlineKeyboardButton> = LazyLock::new(|| {
    InlineKeyboardButton::new(
        "Main menu".to_owned(),
        InlineKeyboardButtonKind::CallbackData(serde_json::to_string(&Action::OpenMain).unwrap()),
    )
});

pub static ADD_ADMIN: LazyLock<InlineKeyboardButton> = LazyLock::new(|| {
    InlineKeyboardButton::new(
        "Add".to_owned(),
        InlineKeyboardButtonKind::CallbackData(serde_json::to_string(&Action::AddAdmin).unwrap()),
    )
});

pub static ADMINS: LazyLock<InlineKeyboardButton> = LazyLock::new(|| {
    InlineKeyboardButton::new(
        "Admins".to_owned(),
        InlineKeyboardButtonKind::CallbackData(serde_json::to_string(&Action::Admins).unwrap()),
    )
});

pub static CONFIGS: LazyLock<InlineKeyboardButton> = LazyLock::new(|| {
    InlineKeyboardButton::new(
        "Configs".to_owned(),
        InlineKeyboardButtonKind::CallbackData(serde_json::to_string(&Action::Configs).unwrap()),
    )
});

pub static CREATE_CONFIG: LazyLock<InlineKeyboardButton> = LazyLock::new(|| {
    InlineKeyboardButton::new(
        "New".to_owned(),
        InlineKeyboardButtonKind::CallbackData(
            serde_json::to_string(&Action::CreateConfig).unwrap(),
        ),
    )
});

pub fn config(c: Config) -> InlineKeyboardButton {
    InlineKeyboardButton::new(
        c.name.to_string(),
        InlineKeyboardButtonKind::CallbackData(
            serde_json::to_string(&Action::Config(c.id)).unwrap(),
        ),
    )
}

pub fn config_rename(c: &Config) -> InlineKeyboardButton {
    InlineKeyboardButton::new(
        "Rename".to_owned(),
        InlineKeyboardButtonKind::CallbackData(
            serde_json::to_string(&Action::RenameConfig(c.id)).unwrap(),
        ),
    )
}

pub fn config_remove(c: &Config) -> InlineKeyboardButton {
    InlineKeyboardButton::new(
        "Remove".to_owned(),
        InlineKeyboardButtonKind::CallbackData(
            serde_json::to_string(&Action::RemoveConfig(c.id)).unwrap(),
        ),
    )
}

pub fn config_file(c: &Config) -> InlineKeyboardButton {
    InlineKeyboardButton::new(
        "Get as file".to_owned(),
        InlineKeyboardButtonKind::CallbackData(
            serde_json::to_string(&Action::GetConfigFile(c.id)).unwrap(),
        ),
    )
}
