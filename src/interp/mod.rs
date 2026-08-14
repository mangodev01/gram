mod display;
mod notify;
mod watcher;
mod commands;
mod init;

use clap::{CommandFactory, FromArgMatches as _};
use color_print::cformat;
use ron::ser::PrettyConfig;
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};

use td_api::*;

use crate::{
    Cli, Command, GramChat, conf::GramConf, interp::commands::{AuthCommands as _, ChatCommands as _, LabelCommands as _, SettingCommands}, util
};

use std::path::Path;

pub struct Interpreter {
    pub client: Arc<parking_lot::Mutex<TdClient>>,
    // pub nesting: Nesting,
    pub chat_folders: Arc<parking_lot::Mutex<Vec<ChatFolderInfo>>>,
    pub conf: Arc<parking_lot::Mutex<GramConf>>,
    pub conf_path: PathBuf,
    pub should_exit: bool,
    pub chats_loaded: bool,
    pub shutdown: Arc<AtomicBool>,
	pub me: Option<i64>,
}

// TODO: implement nesting so you can `enter` a group/supergroup to manage it without
// referencing the IDs every time
// would look something like this:
//
// pub enum Nesting {
//  None,
//  Group {
//		group_label: String,
//  },
//  SupergroupChannel {
//		supergroup_label: String,
//		channel: String,
//  },
// }

impl Interpreter {
    pub fn run(&mut self, line: Command) {
        self.try_set_me();

		let msg = |ch,msg| {
			let mut cli = self.client.lock();

			let chat_id = self.resolve_user(ch);

			cli
				.messages()
				.get_message(chat_id, msg)
		};

        match line {
            Command::Login => self.auth(),
            Command::Logout => self.unauth(),
            Command::Version => println!("gram v0.0.1"),
            Command::Exit => self.should_exit = true,
            Command::Chats { limit, chat_list } => self.chats(limit, chat_list),
            Command::Folders => self.folders(),
            Command::Read { target_chat, limit } => self.read(target_chat, limit),
            Command::Settings { key, value } => self.settings(key, value),
            Command::Send {
                target_chat,
                message,
            } => {
                let joined = message.join(" ");

                match crate::message_content(&joined) {
                    Ok(content) => self.send(target_chat, None, content),
                    Err(e) => util::error!(self.conf.lock().settings.color, "util::error: {}", e),
                }
            },
			Command::Toml {
				target_chat,
				target_message,
			} => {
				let msg = msg(target_chat, target_message);
				
				let toml = toml::to_string_pretty(&msg).unwrap();
				let color = self.conf.lock().settings.color;

				util::success!(color, "{}", toml);
			},
			Command::Ron {
				target_chat,
				target_message,
			} => {
				fn rename_type(value: &mut serde_json::Value) {
					match value {
						serde_json::Value::Object(obj) => {
							if let Some(value) = obj.remove("@type") {
								obj.insert("_type".into(), value);
							}

							for value in obj.values_mut() {
								rename_type(value);
							}
						}
						serde_json::Value::Array(arr) => {
							for value in arr {
								rename_type(value);
							}
						}
						_ => {}
					}
				}

				let msg = msg(target_chat, target_message);

				let mut val = serde_json::to_value(&msg).unwrap();
				rename_type(&mut val);
				
				let ron = ron::ser::to_string_pretty(&val, PrettyConfig::default()).unwrap();
				let color = self.conf.lock().settings.color;

				util::success!(color, "{}", ron);
			},
			Command::Json {
				target_chat,
				target_message,
			} => {
				let msg = msg(target_chat, target_message);

				let json = serde_json::to_string_pretty(&msg).unwrap();
				let color = self.conf.lock().settings.color;

				util::success!(color, "{}", json);
			},
			Command::Forward {
				target_chat,
				target_message,
				to,
			} => {
				self.forward(target_chat, target_message, to, true);
			},
			Command::Repost {
				target_chat,
				target_message,
				to,
			} => {
				self.forward(target_chat, target_message, to, false);
			},
			Command::Text { 
				target_chat,
				message
			} => {
				let joined = message.join(" ");

				self.send(target_chat, None, crate::text_content(&joined));
			}
			Command::Pin { target_chat, target_message } => {
				let mut cli = self.client.lock();

				let chat_id = self.resolve_user(target_chat);

				cli.messages().pin_chat_message(chat_id, target_message, true, false);
			},
			Command::Unpin { target_chat, target_message } => {
				let mut cli = self.client.lock();

				let chat_id = self.resolve_user(target_chat);

				cli.messages().unpin_chat_message(chat_id, target_message);
			},
			Command::Search { q } => {
				let (target_chat, q) = if q.first().map(String::as_str) == Some("in") {
					let chat = q.get(1).cloned().unwrap_or_default();
					let target_chat = Some(match chat.parse::<i64>() {
						Ok(id) => GramChat::ChatID(id),
						Err(_) => GramChat::Label(chat),
					});
					let rest = q.get(2..).unwrap_or(&[]).join(" ");
					(target_chat, rest)
				} else {
					(None, q.join(" "))
				};

				self.search_messages(target_chat, q);
			},
			Command::Gsearch { q } => {
				self.search_messages(None, q.join(""));
			},
			Command::Doc { 
				target_chat,
				message
			} => {
				let joined = message.join(" ");

				let doc_path = Path::new(&joined);

				if !doc_path.exists() {
					util::error!(self.conf.lock().settings.color,
						"document at path {} does not exist.",
						doc_path.to_string_lossy().to_string());

					return;
				}

				self.send(target_chat, None, crate::doc_content(doc_path));
			}
			Command::Me => {
				let mut cli = self.client.lock();

				let me = cli
					.general()
					.get_me();

				let me_full = cli
					.users()
					.get_user_full_info(me.id);

				self.print_user_full(me, me_full);
			}
			Command::Chat { chat } => {
				let mut cli = self.client.lock();

				let chat_id = self.resolve_user(chat);

				let user = cli
					.chats()
					.get_chat(chat_id);

				match user.r#type {
					ChatType::Private { user_id } => {
						let user = cli
							.users()
							.get_user(user_id);

						let user_full = cli
							.users()
							.get_user_full_info(user_id);

						self.print_user_full(user, user_full);
					},
					ChatType::BasicGroup { basic_group_id } => {
						let basic_group = cli.groups()
							.get_basic_group_full_info(basic_group_id);

						let basic_group_partial_info = cli.groups()
							.get_basic_group(basic_group_id);

						self.print_basic_group_meta(basic_group_partial_info, basic_group.clone());

						for member in basic_group.members {
							if let MessageSender::User { user_id } = member.member_id {
								let user = cli
									.users()
									.get_user(user_id);

								let user_full = cli
									.users()
									.get_user_full_info(user_id);

								self.print_user_full(user, user_full);
							}
						}
					},
					ChatType::Supergroup { supergroup_id, .. } => {
                        let super_group_full_info = cli.groups()
                            .get_supergroup_full_info(supergroup_id);

                        let super_group = cli.groups()
                            .get_supergroup(supergroup_id);

                        let members = cli.groups()
                            .get_supergroup_members(supergroup_id, None, 0, 10000);

                        self.print_super_group_meta(super_group, super_group_full_info.clone());

						for member in members.members {
							if let MessageSender::User { user_id } = member.member_id {
								let user = cli
									.users()
									.get_user(user_id);

								let user_full = cli
									.users()
									.get_user_full_info(user_id);

								self.print_user_full(user, user_full);
							}
						}
                    },
					ChatType::Secret { secret_chat_id, user_id } => {
                        let user = cli
                            .users()
                            .get_user(user_id);

                        let user_full = cli
                            .users()
                            .get_user_full_info(user_id);

                        let chat = cli
                            .chats()
                            .get_secret_chat(secret_chat_id);

                        util::info!(self.conf.lock().settings.color, "secret chat layer: {}", chat.layer);

                        self.print_user_full(user, user_full);
                    },
				}
			}
			Command::Delete { 
				target_chat,
				target_message,
			} => {
				let chat = self.resolve_user(target_chat);

				self.client.lock().messages().delete_messages(chat, vec![target_message], true);
			}
			Command::Edit { 
				target_chat,
				target_message,
				message,
			} => {
				let chat = self.resolve_user(target_chat);

				self.client.lock().messages().edit_message_text(chat, target_message, None, InputMessageContent::InputMessageText {
					text: FormattedText {
						text: message.join(" ").to_owned(),
						entities: vec![],
					},
					link_preview_options: None,
					clear_draft: false
				});
			}
            Command::Reply {
                target_chat,
                target_message,
                message,
            } => {
                let joined = message.join(" ");
                match crate::message_content(&joined) {
                    Ok(content) => self.send(target_chat, target_message, content),
                    Err(e) => util::error!(self.conf.lock().settings.color, "util::error: {}", e),
                }
            }
            Command::Label { chat_id, label } => {
                if label.is_none() {
                    self.get_label(chat_id);
                } else if let Some(label) = label {
                    self.label(chat_id, label);
                }
            }
			Command::Script(files) => {
				for file in &files {
					if !Path::new(&file).exists() {
						util::error!(
							self.conf.lock().settings.color,
							"script file {} doesnt exist", file.clone()
						);

						return;
					}
				}

				for file in files {
					let contents = std::fs::read_to_string(&file).unwrap();
					let lines = contents.lines();

					for (i, line) in lines.enumerate() {
						let args = std::iter::once("gram").chain(line.split_whitespace());

						let matches = Cli::command().try_get_matches_from(args);

						if let Ok(matches) = matches {
							let cli = Cli::from_arg_matches(&matches);

							if let Ok(cli) = cli {
								self.run(cli.subcommand);
							} else if let Err(e) = cli {
								util::error!(
									self.conf.lock().settings.color,
									"command conversion error in {}:{} -> {}",
									file,
									i+1,
									e
								);
							}
						} else if let Err(e) = matches {
							util::error!(
								self.conf.lock().settings.color,
								"parse error in {}:{} -> {}",
								file,
								i+1,
								e
							);
						}
					}
				}
			},
            _ => {}
        }
    }

    fn try_set_me(&mut self) {
        if self.me.is_some() {
            return;
        }

        let mut client = self.client.lock();

        if matches!(client.state(), AuthorizationState::Ready) {
            self.me = Some(client.general().get_me().id);
        }
    }

    pub fn prompt(&self) -> String {
        if self.conf.lock().settings.color {
            cformat!("<m!,bold>gram></>")
        } else {
            "gram>".to_string()
        }
    }
}
