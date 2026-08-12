use tabled::{
    Table, builder::Builder, settings::{Alignment, Color, Modify, Style, Width, object::{Columns, Rows}}
};
use td_api::*;

use crate::{interp::Interpreter, util::*};

pub struct GramMessage {
	pub msg: Message,
	pub chat: Chat,
}

impl Interpreter {
    pub(super) fn print_messages(&self, messages: &[GramMessage]) {
        let mut builder = Builder::new();
        builder.push_record(["ID", "Sender", "Date", "Content", "Status"]);
		let conf = self.conf.lock();
        let max_len = conf.settings.max_len_before_shortening;
        for msg in messages.iter().rev() {
			let sender = match &msg.msg.sender_id {
				MessageSender::User { user_id } => {
					let id = format!("user:{}", user_id);

					let name = conf.labels
						.get(user_id)
						.map(|lbl| {
							if self.me == Some(*user_id) {
								"me".to_string()
							} else {
								lbl.name.to_string()
							}
						})
						.unwrap_or_else(|| return shorten(max_len, id.as_str()));

					shorten(max_len, &name)
				}

				MessageSender::Chat { chat_id } => {
					let id = format!("chat:{}", chat_id);

					let name = conf.labels
						.get(chat_id)
						.map(|lbl| {
							if self.me == Some(*chat_id) {
								"me".to_string()
							} else {
								lbl.name.to_string()
							}
						})
						.unwrap_or_else(|| return shorten(max_len, id.as_str()));

					shorten(max_len, &name)
				}
			};

			let read_outbox = msg.chat.last_read_outbox_message_id;

            let date = chrono::DateTime::from_timestamp(msg.msg.date as i64, 0)
                .map(|d| d.with_timezone(&chrono::Local).format("%H:%M:%S").to_string())
                .unwrap_or_else(|| msg.msg.date.to_string());
            let content = match msg.msg.content.as_ref() {
                MessageContent::MessageText { text, .. } => shorten(max_len, &text.text),
                MessageContent::MessagePhoto { .. } => "[photo]".to_string(),
                MessageContent::MessageDocument { document, .. } => {
                    format!("[doc: {}]", document.file_name)
                }
                MessageContent::MessageSticker { sticker, .. } => {
                    format!("[sticker: {}]", sticker.emoji)
                }
                MessageContent::MessageVideo { .. } => "[video]".to_string(),
                MessageContent::MessageAudio { .. } => "[audio]".to_string(),
				MessageContent::MessageCall { unique_id: _, is_video, discard_reason: _, duration } => format!("[{}sec {}call]", duration, if *is_video { "video " } else { "" }),
                _ => "[other]".to_string(),
            };

			let unread = if !msg.msg.is_outgoing {
				""
			} else if msg.msg.sending_state.is_some() {
				"✗"
			} else if read_outbox >= msg.msg.id {
				"✓✓"
			} else {
				"✓"
			};

            builder.push_record([msg.msg.id.to_string(), sender, date, content, unread.to_owned()]);
        }
        let mut table = builder.build();
        Self::init_table(&mut table);
        info!(conf.settings.color, "{}", table);
    }

    pub(super) fn push_chat(
        b: &mut Builder,
        i: usize,
        id: i64,
        kind: &str,
        iid: i64,
        title: String,
        label: String,
        max_len: i32,
        dev: bool,
    ) {
        let title = shorten(max_len, &title);
        let label = shorten(max_len, &label);
        if dev {
            b.push_record([
                i.to_string(),
                id.to_string(),
                kind.to_string(),
                iid.to_string(),
                title,
                label,
            ]);
        } else {
            b.push_record([
                i.to_string(),
                id.to_string(),
                kind.to_string(),
                title,
                label,
            ]);
        }
    }

	pub(super) fn init_table(table: &mut Table) {
		const COL_ID: usize = 12;
		const COL_SENDER: usize = 15;
		const COL_DATE: usize = 8;
		const COL_BORDERS: usize = 13;
		const FIXED: usize = COL_ID + COL_SENDER + COL_DATE + COL_BORDERS;

		let term_width = terminal_size::terminal_size()
			.map(|(w, _)| w.0 as usize)
			.unwrap_or(120)
			.min(120);

		let content_width = term_width.saturating_sub(FIXED).max(20);

		table
			.with(Color::BOLD)
			.with(Style::modern_rounded())
			.with(Modify::new(Columns::last()).with(Width::wrap(content_width).keep_words(true)))
			.with(Modify::new(Rows::first()).with(Alignment::center()));
	}
}
