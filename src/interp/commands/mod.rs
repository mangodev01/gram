use crate::{GramChat, GramMessageContent};

mod chat;
mod labels;
mod auth;
mod settings;

pub(super) trait ChatCommands {
    fn chats(&mut self, limit: i32, chat_list: td_api::ChatList);
    fn read(&mut self, target_chat: GramChat, limit: i32);

	fn send(&mut self, target_chat: GramChat, reply_to: Option<i64>, message: GramMessageContent);
	fn resolve_user(&self, target_chat: GramChat) -> i64;

	fn folders(&self);
}

pub(super) trait AuthCommands {
    fn auth(&mut self);
    fn unauth(&mut self);
}

pub(super) trait LabelCommands {
	fn label(&mut self, chat_id: i64, name: String);
	fn get_label(&self, chat_id: i64);
}

pub(super) trait SettingCommands {
    fn settings(&mut self, key: Option<String>, value: Option<String>);

    fn refresh_conf(&mut self);

    fn save_conf(&self);
}
