use tabled::builder::Builder;
use td_api::*;

use crate::GramChat;
use crate::GramMessageContent;
use crate::Interpreter;

use crate::interp::commands::ChatCommands;
use crate::interp::display::GramMessage;
use crate::util;
use crate::util::{log_err, tri};

impl Interpreter {
    pub(crate) fn ensure_chats_loaded(&mut self) -> bool {
        if self.chats_loaded {
            return true;
        }

        let mut client = self.client.lock();
        client.wait_for_state(AuthorizationState::Ready);
        let result = client.chats().get_chats(None, 200);

        match result {
            Ok(_) => {
                self.chats_loaded = true;
                true
            }
            Err(err) => {
                util::error!(
                    self.conf.lock().settings.color,
                    "telegram error {}: {}",
                    err.code,
                    err.message
                );
                false
            }
        }
    }
}

impl ChatCommands for Interpreter {
    fn chats(&mut self, limit: i32, chat_list: ChatList) {
        if limit < 1 {
			let conf = self.conf.lock();

            util::error!(conf.settings.color, "chat limit must be positive.");
            return;
        }
        let mut client = self.client.lock();
        let mut builder = Builder::new();

        let conf = self.conf.lock();
        let max_len = conf.settings.max_len_before_shortening;
        let dev = conf.settings.dev;

        if dev {
            builder.push_record(["#", "Id", "Type", "Inner ID", "Title", "Label?"]);
        } else {
            builder.push_record(["#", "Id", "Type", "Title", "Label?"]);
        }
        drop(conf);

        let chats = tri!(self, client.chats().get_chats(Some(chat_list), limit));

        self.chats_loaded = true;

        for (i, id) in chats.chat_ids.iter().enumerate() {
            let label = self
                .conf
                .lock()
                .labels
                .get(id)
                .map(|l| l.name.clone())
                .unwrap_or("".to_string());
            let chat = tri!(self, client.chats().get_chat(*id));
            let (kind, iid) = match chat.r#type {
                ChatType::Private { user_id: iid } => ("private", iid),
                ChatType::BasicGroup {
                    basic_group_id: iid,
                } => ("bagroup", iid),
                ChatType::Supergroup {
                    supergroup_id: iid, ..
                } => ("sugroup", iid),
                ChatType::Secret { user_id: iid, .. } => ("secretc", iid),
            };

            Self::push_chat(
                &mut builder,
                i,
                *id,
                kind,
                iid,
                chat.title,
                label,
                max_len,
                dev,
            );
        }
        let mut table = builder.build();
        Self::init_table(&mut table);
        util::info!(
            self.conf.lock().settings.color,
            "theres a total of {} chats.", chats.total_count
        );
        util::info!(self.conf.lock().settings.color, "{}", table);
    }

    fn read(&mut self, target_chat: GramChat, limit: i32) {
        if limit < 1 {
            util::error!(
                self.conf.lock().settings.color,
                "message limit must be positive."
            );
            return;
        }

        if !self.ensure_chats_loaded() {
            return;
        }

        let chat_id = self.resolve_user(target_chat);

        if chat_id == -1 {
            return;
        }

        let mut client = self.client.lock();
        let chat = tri!(self, client.chats().get_chat(chat_id));

        let requested = limit as usize;
        let mut collected = Vec::with_capacity(requested);
        let mut seen_ids = std::collections::HashSet::with_capacity(requested);
        let mut from_message_id = 0i64;
        let mut no_progress_attempts = 0u8;

        while collected.len() < requested {
            // Subsequent pages can include the cursor message, so request one
            // extra item when possible and deduplicate by message ID.
            let remaining = requested - collected.len();
            let page_limit = (remaining + usize::from(from_message_id != 0))
                .min(100) as i32;

            let history = tri!(
                self,
                client.chats().get_chat_history(
                    chat_id,
                    from_message_id,
                    0,
                    page_limit,
                    false
                )
            );

            let page = history.messages.unwrap_or_default();

            if page.is_empty() {
                break;
            }

            let next_from_message_id = page.last().map(|message| message.id);
            let previous_len = collected.len();

            for message in page {
                if seen_ids.insert(message.id) {
                    collected.push(message);

                    if collected.len() == requested {
                        break;
                    }
                }
            }

            let Some(next_from_message_id) = next_from_message_id else {
                break;
            };

            if collected.len() == previous_len {
                no_progress_attempts += 1;

                // TDLib can initially return only the already-known boundary
                // message while older history is being loaded. Give it a few
                // more requests, but keep the loop bounded at the end of the
                // chat history.
                if no_progress_attempts >= 3 {
                    break;
                }
            } else {
                no_progress_attempts = 0;
            }

            from_message_id = next_from_message_id;
        }

        util::info!(
            self.conf.lock().settings.color,
            "got {} messages.",
            collected.len()
        );

        let messages: Vec<GramMessage> = collected
            .into_iter()
            .map(|msg| GramMessage {
                msg,
                chat: chat.clone(),
            })
            .collect();

        self.print_messages(&messages);
    }

	fn send(
		&mut self,
		target_chat: GramChat,
		reply_to: Option<i64>,
		message: GramMessageContent,
	) {
		if !self.ensure_chats_loaded() {
			return;
		}

		let mut client = self.client.lock();
		let chat_id = self.resolve_user(target_chat);

		if chat_id == -1 {
			return;
		}

		let reply_to = reply_to.map(|message_id| InputMessageReplyTo::Message {
			message_id,
			quote: None,
			checklist_task_id: 0,
			poll_option_id: "".to_string(),
		});

		let content = message.0;

		log_err!(
			self,
			client
				.messages()
				.send_message(chat_id, None, reply_to, None, None, content)
		);
	}

	fn forward(
		&mut self,
		target_chat: GramChat,
		target_msg: i64,
		to: GramChat,
		forward: bool
	) {
		if !self.ensure_chats_loaded() {
			return;
		}

		let mut client = self.client.lock();
		let chat_id = self.resolve_user(target_chat);
		let msg_to_forward = tri!(self, client.messages().get_message(chat_id, target_msg));
		let to = self.resolve_user(to);

		if forward {
			log_err!(
				self,
				client
					.messages()
					.forward_messages(to, None, chat_id, vec![target_msg], None, false, false)
			);
		} else {
			let imc = util::message_content_to_input_message_content(*msg_to_forward.content);

			if imc.is_none() {
				util::error!(
					self.conf.lock().settings.color,
					"error: couldn't convert message content to input message content"
				);

				return;
			}

			let imc = imc.unwrap();

			log_err!(self, client.messages().send_message(to, None, None, None, None, imc));
		}
	}


    fn resolve_user(&self, target_chat: GramChat) -> i64 {
        match target_chat {
            GramChat::Label(name) => {
                let key = self
                    .conf
                    .lock()
                    .labels
                    .iter()
                    .find(|(_, v)| v.name == name)
                    .map(|(k, _)| *k);

                if key.is_none() {
                    util::error!(
                        self.conf.lock().settings.color,
                        "error: target chat label is not attached to any known chat"
                    );
                    return -1;
                }

                key.unwrap()
            }
            GramChat::ChatId(id) => id,
        }
    }

	fn folders(&self) {
		let mut builder = Builder::new();
		builder.push_record(["ID", "Name"]);

		let folders = self.chat_folders.lock();
		let max_len = self.conf.lock().settings.max_len_before_shortening;
		for folder in folders.iter() {
			builder.push_record([
				folder.id.to_string(),
				util::shorten(max_len, &folder.name.text.text),
			]);
		}

		let mut table = builder.build();
		Self::init_table(&mut table);
		util::info!(self.conf.lock().settings.color, "{}", table);
	}

	fn search_messages(&mut self, target_chat: Option<GramChat>, q: String) {
		let mut client = self.client.lock();

		if let Some(chat) = target_chat {
			let chat_id = self.resolve_user(chat);

			let chat = tri!(self, client.chats().get_chat(chat_id));

			let found = tri!(self, client.messages().search_chat_messages(
				chat_id,
				None,
				q,
				None,
				0i64,
				0i32,
				200i32,
				None
			));

			let msgs = found.messages.as_slice().iter().map(|msg| {
				GramMessage { msg: msg.clone(), chat: chat.clone() }
			}).collect::<Vec<_>>();

			self.print_messages(msgs.as_slice());
		} else {
			let found = tri!(self, client.messages().search_messages(
				None,
				q,
				String::new(),
				200i32,
				None,
				None,
				0i32,
				0i32
			));

			let mut msgs = Vec::with_capacity(found.messages.len());

			for msg in found.messages.iter() {
				let chat_id = msg.chat_id;

				let chat = tri!(self, client.chats().get_chat(chat_id));

				msgs.push(GramMessage { msg: msg.clone(), chat });
			}

			self.print_messages(msgs.as_slice());
		}
	}
}
