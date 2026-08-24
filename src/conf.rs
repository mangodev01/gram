use std::collections::HashMap;
use std::hash_map;

use macros::settings;
use td_api::{Chat, ChatFolderInfo, User};

// honestly i dont even know why im implementing serde::Serialize for these
// its not even necessary
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct GramConf {
    /// map between telegram chat id and gram labels so you can later reference gram labels for
    /// more convenience
	#[serde(default = "default_labels")]
    pub labels: HashMap<i64, GramLabel>,

	#[serde(default = "default_settings")]
    pub settings: GramSettings,

    #[serde(default = "default_encryption")]
    pub encryption: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct GramLabel {
    /// name of the gram label
    pub name: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ChatFoldersCache {
    pub folders: Vec<ChatFolderInfo>,
}

settings! {
    /// user sender mode to display sender users with
    pub user_sender_mode: UserSenderMode = default_user_sender_mode,

    /// chat sender mode to display sender chats with
    pub chat_sender_mode: ChatSenderMode = default_chat_sender_mode,

    /// max length before shortening (appending '...') message / username / other text
    /// -1 means no shortening at all
    pub max_len_before_shortening: i32 = default_max_length,

    pub color: bool = default_true,

    pub dev: bool = default_false,
}

#[allow(dead_code)]
fn default_encryption() -> Option<String> {
    None
}

#[allow(dead_code)]
fn default_true() -> bool {
    true
}

#[allow(dead_code)]
fn default_false() -> bool {
    false
}

#[allow(dead_code)]
fn default_labels() -> HashMap<i64, GramLabel> {
	hash_map! {}
}

#[allow(dead_code)]
fn default_settings() -> GramSettings {
	GramSettings::new()
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UserSenderMode {
    /// labels user senders by first and last name
    FirstAndLastName,

    /// labels user senders by first name only
    FirstName,

    /// labels user senders by last name only
    LastName,

    /// labels user senders by telegram user ID
    UserId,

    /// labels user senders by custom pattern
    /// example of custom pattern usage:
    /// ```
    /// {{first_name}} {{last_name}} ({{user_id}})
    /// ```
    Custom(String),
}

impl UserSenderMode {
    pub fn with_user(&self, user: User) -> String {
        match self {
            UserSenderMode::FirstAndLastName => {
                format!("{} {}", user.first_name, user.last_name)
            }
            UserSenderMode::FirstName => user.first_name,
            UserSenderMode::LastName => user.last_name,
            UserSenderMode::UserId => user.id.to_string(),
            UserSenderMode::Custom(fmt) => fmt
                .replace("{{first_name}}", &user.first_name)
                .replace("{{last_name}}", &user.last_name)
                .replace("{{user_id}}", &user.id.to_string()),
        }
    }
}

#[allow(dead_code)]
fn default_user_sender_mode() -> UserSenderMode {
    UserSenderMode::FirstAndLastName
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatSenderMode {
    /// labels chat senders by gram label
    Label,

    /// labels chat senders by chat title
    Title,

    /// labels chat senders by telegram chat ID
    ChatId,

    /// labels chat senders by custom pattern
    /// example of custom pattern usage:
    /// ```
    /// {{title}}/{{label}}/{{chat_id}}
    /// ```
    Custom(String),
}

impl ChatSenderMode {
    pub fn with_chat(&self, conf: GramConf, chat: Chat) -> String {
        match self {
            ChatSenderMode::Label => conf
                .labels
                .get(&chat.id)
                .map(|x| x.name.clone())
                .unwrap_or(chat.title),
            ChatSenderMode::Title => chat.title,
            ChatSenderMode::ChatId => chat.id.to_string(),
            ChatSenderMode::Custom(fmt) => fmt
                .replace("{{title}}", &chat.title)
                .replace(
                    "{{label}}",
                    &conf
                        .labels
                        .get(&chat.id)
                        .map(|x| x.name.clone())
                        .unwrap_or(chat.title),
                )
                .replace("{{chat_id}}", &chat.id.to_string()),
        }
    }
}

#[allow(dead_code)]
fn default_chat_sender_mode() -> ChatSenderMode {
    ChatSenderMode::Title
}

#[allow(dead_code)]
fn default_max_length() -> i32 {
    -1
}
