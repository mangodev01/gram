#![feature(decl_macro, hash_map_macro, strip_circumfix)]

use clap::{
    builder::{styling::AnsiColor, Styles},
    ColorChoice, CommandFactory, FromArgMatches,
};

use rustyline::{config::Configurer as _, hint::HistoryHinter};
use rustyline::history::DefaultHistory;
use rustyline::error::ReadlineError;
use td_api::{ChatList, FormattedText, InputFile, InputMessageContent};

use crate::{
    interp::Interpreter,
    util::{error, info},
};

mod conf;
mod interp;
mod util;
mod completion;

#[derive(clap::Parser)]
struct Cli {
    #[command(subcommand)]
    pub subcommand: Command,
}

#[derive(Clone, PartialEq)]
pub enum GramChat {
    Label(String),
    ChatID(i64),
}

impl std::str::FromStr for GramChat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(id) = s.parse::<i64>() {
            Ok(GramChat::ChatID(id))
        } else {
            Ok(GramChat::Label(s.to_string()))
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct GramMessageContent(InputMessageContent);

fn resolve_path(val: &str) -> Result<std::path::PathBuf, String> {
    let path = std::path::Path::new(val);

    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };

    std::fs::canonicalize(&abs).map_err(|e| e.to_string())
}

fn text_content(s: &str) -> GramMessageContent {
    GramMessageContent(InputMessageContent::InputMessageText {
        text: FormattedText {
            text: s.to_string(),
            entities: vec![],
        },
        clear_draft: true,
        link_preview_options: None,
    })
}

fn doc_content(path: &std::path::Path) -> GramMessageContent {
    GramMessageContent(InputMessageContent::InputMessageDocument {
        document: InputFile::Local {
            path: path.to_string_lossy().to_string(),
        },
        thumbnail: None,
        disable_content_type_detection: false,
        caption: None,
    })
}

pub fn message_content(s: &str) -> Result<GramMessageContent, String> {
    if let Some(txt) = s.strip_circumfix("\"", "\"") {
        if let Ok(path) = resolve_path(txt) {
            if path.try_exists().is_ok_and(|x| x) {
                return Ok(text_content(txt));
            }
        }
    }

    if let Ok(path) = resolve_path(s) {
        if path.try_exists().is_ok_and(|x| x) {
            return Ok(doc_content(&path));
        }
    }

    Ok(text_content(s))
}

#[derive(clap::Subcommand, Clone, PartialEq)]
pub enum Command {
    #[clap(about = "authenticate with telegram")]
    Login,

    #[clap(about = "deauthenticate from telegram")]
    Logout,

    #[clap(about = "print gram version")]
    Version,

    #[clap(about = "list chat folders")]
    Folders,

    #[clap(about = "clear repl - only works in repl mode")]
    Clear,

    #[clap(about = "send a message to a chat (automatically determines what type the message is)")]
    Send {
        target_chat: GramChat,
        #[arg(trailing_var_arg = true, num_args = 1..)]
        message: Vec<String>,
    },

    #[clap(about = "send a TEXT message to a chat (force text)")]
    Text {
        target_chat: GramChat,
        #[arg(trailing_var_arg = true, num_args = 1..)]
        message: Vec<String>,
    },

    #[clap(about = "send a DOCUMENT to a chat (force document)")]
    Doc {
        target_chat: GramChat,
        #[arg(trailing_var_arg = true, num_args = 1..)]
        message: Vec<String>,
    },

	#[clap(about = "fetch info of yourself (as the user)")]
	Me,

	#[clap(about = "fetch info of a chat")]
	Chat {
		chat: GramChat
	},

	#[clap(about = "delete a message in a chat")]
	Delete {
		target_chat: GramChat,
        target_message: i64,
	},

	#[clap(about = "edit a message in a chat")]
	Edit {
		target_chat: GramChat,
        target_message: i64,

        #[arg(trailing_var_arg = true, num_args = 1..)]
        message: Vec<String>,
	},

    #[clap(about = "reply to a message in a chat")]
    Reply {
        target_chat: GramChat,

        target_message: Option<i64>,

        #[arg(trailing_var_arg = true, num_args = 1..)]
        message: Vec<String>,
    },

    #[clap(about = "read recent messages of a chat")]
    Read {
        target_chat: GramChat,
        #[arg(default_value = "10")]
        limit: i32,
    },

    #[clap(about = "get/set a label for a chat")]
    Label {
        /// the telegram chat id you wanna label
        #[arg(allow_hyphen_values = true)]
        chat_id: i64,

        /// the label you wanna give that telegram chat id (None if user wants to GET a label of a
        /// chat)
        label: Option<String>,
    },

    #[clap(about = "list/get/set gram settings")]
    Settings {
        // if only key is provided, get
        // if both key and value are provided, set
        // if none are provided, list
        key: Option<String>,
        #[arg(allow_hyphen_values = true)]
        value: Option<String>,
    },

    #[clap(alias = "quit", about = "exit gram")]
    Exit,

    #[clap(about = "list chats")]
    Chats {
        #[arg(default_value = "10")]
        limit: i32,

        #[arg(value_parser = chat_list, default_value = "main")]
        chat_list: ChatList,
    },

    #[clap(about = "show message toml")]
	Toml {
        target_chat: GramChat,

        target_message: i64,
	},

    #[clap(about = "show message ron")]
	Ron {
        target_chat: GramChat,

        target_message: i64,
	},

    #[clap(about = "show message json")]
	Json {
        target_chat: GramChat,

        target_message: i64,
	},

    #[clap(about = "forward a message from chat to chat")]
	Forward {
        target_chat: GramChat,

        target_message: i64,

		to: GramChat
	},

	#[clap(about = "pins a message")]
	Pin {
		target_chat: GramChat,

		target_message: i64
	},

	#[clap(about = "unpins a message")]
	Unpin {
		target_chat: GramChat,

		target_message: i64
	},

	#[clap(
		about = "searches for messages either globally or in a chat",
		after_help = "searches globally across all chats by default\n\
prefix with 'in <chat>' to search within a specific chat:\n\
\n\
search hello world        search all chats for \"hello world\"\n\
search in bob hello       search only in bob's chat for \"hello\"\n\
\n\
use 'gsearch' instead of 'search' to force a global search\n\
even if your query happens to start with 'in'"
	)]
	Search {
		#[arg(trailing_var_arg = true, allow_hyphen_values = true, value_name = "[in CHAT] QUERY...")]
		q: Vec<String>
	},

	#[clap(about = "searches for messages globally")]
	Gsearch {
		#[arg(trailing_var_arg = true, allow_hyphen_values = true)]
		q: Vec<String>
	},

    #[clap(about = "forward a message from chat to chat WITHOUT leaving a trace of the message being forwarded")]
	Repost {
        target_chat: GramChat,

        target_message: i64,

		to: GramChat
	},

	#[command(external_subcommand)]
	Script(Vec<String>),
}

pub fn chat_list(s: &str) -> Result<ChatList, String> {
    if s == "m" || s == "main" || s == "ChatList::Main" {
        return Ok(ChatList::Main);
    } else if s == "a" || s == "archive" || s == "ChatList::Archive" {
        return Ok(ChatList::Archive);
    } else {
        if s.starts_with("folder/") || s.starts_with("f/") || s.starts_with("ChatList::Folder/") {
            let (_, folder_id) = s.split_once("/").unwrap();
            let folder_id_num = folder_id.parse::<i32>();

            if let Ok(chat_folder_id) = folder_id_num {
                return Ok(ChatList::Folder { chat_folder_id });
            }
        }
    }

    Err("invalid chat list".to_string())
}

fn main() {
    let mut interp = Interpreter::new();

    let styles = Styles::styled()
        .header(AnsiColor::BrightGreen.on_default().bold())
        .usage(AnsiColor::BrightGreen.on_default().bold())
        .literal(AnsiColor::BrightCyan.on_default().bold())
        .placeholder(AnsiColor::BrightCyan.on_default());

    let mut line_empty = true;

    if std::env::args_os().len() == 1 {
        let mut ed = rustyline::Editor::<completion::GramHelper, DefaultHistory>::new().unwrap();

		let helper = completion::GramHelper {
			hinter: HistoryHinter::default(),
		};

		ed.set_helper(Some(helper));
		ed.set_completion_type(rustyline::CompletionType::Circular);

        loop {
            let line = match ed.readline(&interp.prompt()) {
                Ok(line) => line,
                Err(ReadlineError::Interrupted) => {
                    if !line_empty {
                        info!(
                            interp.conf.lock().settings.color,
                            "use `exit` or `quit` to exit."
                        );
                    }

                    "".to_string()
                }
                Err(ReadlineError::Eof) => break,
                Err(e) => {
                    error!(interp.conf.lock().settings.color, "{e}");
                    break;
                }
            };

            if line.trim().is_empty() {
                line_empty = true;
                continue;
            }

            line_empty = false;

            ed.add_history_entry(&line).unwrap();

            let matches = match Cli::command()
                .styles(styles.clone())
                .color(if interp.conf.lock().settings.color {
                    ColorChoice::Always
                } else {
                    ColorChoice::Never
                })
                .try_get_matches_from(std::iter::once("gram").chain(line.split_whitespace()))
            {
                Ok(cli) => cli,
                Err(e) => {
                    // allow clap to manage its own colors without forcing them on using
                    // ceprintln
                    e.print().unwrap();

                    continue;
                }
            };

            let parsed = match Cli::from_arg_matches(&matches) {
                Ok(parsed) => parsed,
                Err(e) => {
                    error!(interp.conf.lock().settings.color, "{e}");
                    continue;
                }
            };

            if matches!(parsed.subcommand, Command::Clear) {
                ed.clear_screen().unwrap();
                continue;
            }

            interp.run(parsed.subcommand);

            if interp.should_exit {
                return;
            }
        }
    } else {
        let matches = Cli::command()
            .styles(styles)
            .color(if interp.conf.lock().settings.color {
                ColorChoice::Always
            } else {
                ColorChoice::Never
            })
            .get_matches();

        let cli = Cli::from_arg_matches(&matches).unwrap();

        if matches!(cli.subcommand, Command::Clear) {
            error!(
                interp.conf.lock().settings.color,
                "clear only works in REPL"
            );
            return;
        }

        interp.run(cli.subcommand);
    }
}

