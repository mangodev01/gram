mod display;
mod notify;
mod watcher;
mod commands;
mod init;

use clap::{CommandFactory, FromArgMatches as _};
use color_print::cformat;
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};

use td_api::*;

use crate::{
    Cli, Command, conf::GramConf, interp::commands::{AuthCommands as _, ChatCommands as _, LabelCommands as _, SettingCommands}, util
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
            }
			Command::Text { 
				target_chat,
				message
			} => {
				let joined = message.join(" ");

				self.send(target_chat, None, crate::text_content(&joined));
			}
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
            Command::Reply {
                target_chat,
                target_message,
                message,
            } => {
                let joined = message.join(" ");
                match crate::message_content(&joined) {
                    Ok(content) => self.send(target_chat, Some(target_message), content),
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

    pub fn prompt(&self) -> String {
        if self.conf.lock().settings.color {
            cformat!("<m!,bold>gram></>")
        } else {
            "gram>".to_string()
        }
    }
}
