use std::path::PathBuf;
use std::sync::Arc;

use td_api::{*, extra::*};

use crate::util::*;

use crate::{conf::{ChatFoldersCache, GramConf}, interp::Interpreter, util};

impl Interpreter {
    pub(super) fn notify(
        conf: Arc<parking_lot::Mutex<GramConf>>,
        client: Arc<parking_lot::Mutex<TdClient>>,
        message: Message,
    ) {
        let mut client = client.lock();
        let conf = conf.lock().clone();

        if message.is_outgoing {
            return;
        }

        let max_len = conf.settings.max_len_before_shortening;

        let sender = match message.sender_id {
            MessageSender::User { user_id } => {
                let user = match client.users().get_user(user_id) {
                    Ok(user) => user,
                    Err(err) => {
                        util::error!(
                            conf.settings.color,
                            "telegram error {}: {}",
                            err.code,
                            err.message
                        );
                        return;
                    }
                };

                util::shorten(
                    max_len,
                    &conf.settings.user_sender_mode.with_user(user.clone()),
                )
            }
            MessageSender::Chat { chat_id } => {
                let chat = match client.chats().get_chat(chat_id) {
                    Ok(chat) => chat,
                    Err(err) => {
                        util::error!(
                            conf.settings.color,
                            "telegram error {}: {}",
                            err.code,
                            err.message
                        );
                        return;
                    }
                };

                util::shorten(
                    max_len,
                    &conf
                        .settings
                        .chat_sender_mode
                        .with_chat(conf.clone(), chat.clone()),
                )
            }
        };

        let content_str = match *message.content {
            MessageContent::MessageText { text, .. } => util::shorten(max_len, &text.text),
            MessageContent::MessageDocument { document, .. } => {
                util::shorten(max_len, &document.file_name)
            }
            MessageContent::MessageVideo { video, .. } => util::shorten(max_len, &video.file_name),
            MessageContent::MessagePhoto { caption, .. } => {
                util::shorten(max_len, &format!("<video> {}", caption.text))
            }
            _ => util::shorten(
                max_len,
                &format!("unsupported message type {:?}", message.content),
            ),
        };

        std::thread::spawn(move || {
            let _ = notify_rust::Notification::new()
                .summary(&sender)
                .body(&content_str)
                .show();
        });
    }


    pub(super) fn spawn_threads(
        client: Arc<parking_lot::Mutex<TdClient>>,
        conf: Arc<parking_lot::Mutex<GramConf>>,
        recvs: TdReceivers,
        out_chat_folders: Arc<parking_lot::Mutex<Vec<ChatFolderInfo>>>,
        chat_folders_path: PathBuf,
    ) {
        let client_weak = Arc::downgrade(&client);
        let conf_update = Arc::clone(&conf);

        // update thread
		std::thread::spawn(move || {
			for update in &recvs.update_rx {
				if let Update::ChatFolders { chat_folders, .. } = update {
					*out_chat_folders.lock() = chat_folders.clone();

					let idk = toml::to_string_pretty(&ChatFoldersCache {
						folders: chat_folders,
					});
					if let Ok(ser) = idk {
						let _ = std::fs::write(&chat_folders_path, ser);
					} else {
						error!(conf_update.lock().settings.color, "{:#?}", idk.unwrap_err());
					}
				} else if let Update::NewMessage { message } = update {
					if let Some(client) = client_weak.upgrade() {
						Self::notify(conf_update.clone(), client, message);
					}
				}
			}
		});
    }
}
