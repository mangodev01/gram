use color_print::cformat;
use notify::{Event, RecursiveMode, Watcher as _};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
};
use tabled::{
    builder::Builder,
    settings::{object::Rows, Alignment, Color, Modify, Style},
    Table,
};

use directories::ProjectDirs;
use td_api::{
    extra::TdReceivers, params::TdParamsBuilder, ChatFolderInfo, ChatList, ChatType, Encryption,
    InputMessageReplyTo, Message, MessageContent, MessageSender, TdClient, Update,
};

use crate::{
    conf::{ChatFoldersCache, GramConf, GramLabel},
    util::{error, info, success},
    Command, GramChat, GramMessageContent,
};

const DEFAULT_CONFIG: &str = include_str!("../default_conf.toml");

fn shorten(max_len_before_shortening: i32, s: &str) -> String {
    if max_len_before_shortening < 0 {
        return s.to_string();
    }
    let max_len = max_len_before_shortening as usize;
    if s.chars().count() > max_len {
        s.chars().take(max_len).collect::<String>() + "..."
    } else {
        s.to_string()
    }
}

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
    pub fn new() -> Self {
        let cur = std::env::current_dir().unwrap();
        let db = cur.join("db");
        let files = cur.join("files");

        let params = TdParamsBuilder::new()
            .creds_from_env("ID", "HASH")
            .db_dir(db)
            .files_dir(files)
            .app_version("0.0.1".to_string())
            .use_test_dc(false)
            .enable_storage_optimiser(true)
            .use_secret_chats(true)
            .device_model("gram".to_string())
            .sys_lang_code("en".to_string())
            .build();

        // TODO: make Encryption::Yes an option
        let (mut client, recvs) = TdClient::new(params, Encryption::No);

        client.general().set_log_verbosity_level(0);

        client.set_verbosity(0);
        client.set_log_to_file(false);

        let client = Arc::new(parking_lot::Mutex::new(client));

        let dirs =
            ProjectDirs::from("me", "illia", "gram").expect("failed to get project directories");

        let _ = std::fs::create_dir_all(dirs.cache_dir());
        let _ = std::fs::create_dir_all(dirs.config_dir());

        let chat_folders_path = dirs.cache_dir().join("chat_folders.toml");
        let chat_folders = Arc::new(parking_lot::Mutex::new(
            std::fs::read_to_string(&chat_folders_path)
                .ok()
                .and_then(|s| toml::from_str::<ChatFoldersCache>(&s).ok())
                .map(|c| c.folders)
                .unwrap_or_default(),
        ));

        let conf_path = dirs.config_dir().join("conf.toml");

        if !std::fs::exists(&conf_path).is_ok_and(|x| x) {
            let _ = std::fs::write(&conf_path, DEFAULT_CONFIG);
        }

        let read_conf = std::fs::read_to_string(&conf_path).unwrap();

        let conf: GramConf = toml::from_str(&read_conf).expect("failed to parse config");

        let conf = Arc::new(parking_lot::Mutex::new(conf));
        let shutdown = Arc::new(AtomicBool::new(false));

        Self::spawn_threads(
            Arc::clone(&client),
            conf.clone(),
            recvs,
            Arc::clone(&chat_folders),
            dirs.cache_dir().join("chat_folders.toml"),
        );

        // conf watcher thread
        {
            let conf = Arc::clone(&conf);
            let conf_path = conf_path.clone();
            let watch_dir = conf_path.parent().unwrap().to_path_buf();
            let shutdown = Arc::clone(&shutdown);
            std::thread::spawn(move || {
                let conf_watcher = Arc::clone(&conf);
                let conf_log = Arc::clone(&conf);
                let conf_path = conf_path.clone();

                let mut watcher =
                    match notify::recommended_watcher(move |res: notify::Result<Event>| {
                        if let Ok(event) = res {
                            if event.paths.iter().any(|p| p == &conf_path) {
                                let read = match std::fs::read_to_string(&conf_path) {
                                    Ok(r) => r,
                                    Err(_) => return,
                                };
                                match toml::from_str(&read) {
                                    Ok(cfg) => *conf_watcher.lock() = cfg,
                                    Err(_) => {}
                                }
                            }
                        }
                    }) {
                        Ok(w) => w,
                        Err(e) => {
                            error!(
                                conf_log.lock().settings.color,
                                "failed to create file watcher: {e}"
                            );
                            return;
                        }
                    };

                if let Err(e) = watcher.watch(&watch_dir, RecursiveMode::NonRecursive) {
                    error!(
                        conf_log.lock().settings.color,
                        "failed to watch config directory: {e}"
                    );
                    return;
                }

                loop {
                    if shutdown.load(Ordering::Relaxed) {
                        break;
                    }
                    std::thread::park();
                }
            });
        }

        // poll thread
        {
            let client = Arc::clone(&client);
            let shutdown = Arc::clone(&shutdown);
            std::thread::spawn(move || {
                while !shutdown.load(Ordering::Relaxed) {
                    client.lock().poll();
                }
            });
        }

        Self {
            client,
            // nesting: Nesting::None,
            chat_folders,
            conf,
            conf_path,
            should_exit: false,
            chats_loaded: false,
            shutdown,
        }
    }

    pub fn resolve_user(&self, target_chat: GramChat) -> i64 {
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
                    error!(
                        self.conf.lock().settings.color,
                        "error: target chat label is not attached to any known chat"
                    );
                    return -1;
                }

                key.unwrap()
            }
            GramChat::ChatID(id) => id,
        }
    }

    pub fn label(&mut self, chat_id: i64, name: String) {
        let _ = self.conf.lock().labels.insert(chat_id, GramLabel { name });
        self.save_conf();
    }

    pub fn get_label(&self, chat_id: i64) {
        let conf = self.conf.lock();

        let label = conf.labels.get(&chat_id);

        if let Some(label) = label {
            success!(conf.settings.color, "chat has label {}", label.name);
        } else {
            error!(
                conf.settings.color,
                "chat doesn't have label attached. attach one by using `label {{chat}} {{label}}`"
            );
        }
    }

    pub fn auth(&mut self) {
        let mut client = self.client.lock();

        client.authenticate_with_console().unwrap();
    }

    pub fn unauth(&mut self) {
        let _ = std::fs::remove_dir_all("db");
    }

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
                    Err(e) => error!(self.conf.lock().settings.color, "error: {}", e),
                }
            }
            Command::Reply {
                target_chat,
                target_message,
                message,
            } => {
                let joined = message.join(" ");
                match crate::message_content(&joined) {
                    Ok(content) => self.send(target_chat, Some(target_message), content),
                    Err(e) => error!(self.conf.lock().settings.color, "error: {}", e),
                }
            }
            Command::Label { chat_id, label } => {
                if label.is_none() {
                    self.get_label(chat_id);
                } else if let Some(label) = label {
                    self.label(chat_id, label);
                }
            }
            _ => {}
        }
    }

    pub fn settings(&mut self, key: Option<String>, value: Option<String>) {
        self.refresh_conf();

        match (key, value) {
            (None, None) => {
                // list
                let conf = self.conf.lock();

                let mut builder = Builder::new();

                for (k, v) in &conf.settings {
                    builder.push_record([k, &v]);
                }

                let mut table = builder.build();
                Self::init_table(&mut table);
                info!(conf.settings.color, "{}", table);
            }
            (Some(k), None) => {
                // get
                let conf = self.conf.lock();

                let v = match conf.settings.get(&k) {
                    Ok(v) => v,
                    Err(err) => {
                        error!(
                            conf.settings.color,
                            "error while getting setting {k}: {err}"
                        );
                        return;
                    }
                };

                info!(conf.settings.color, "{k}={v}");
            }
            (Some(k), Some(v)) => {
                // set
                let mut conf = self.conf.lock();

                if let Err(err) = conf.settings.set(&k, &v) {
                    error!(
                        conf.settings.color,
                        "error while setting setting {k} to \"{v}\": {err}"
                    );
                    return;
                }

                info!(conf.settings.color, "{k}={v}");

                drop(conf);
                self.save_conf();

                self.refresh_conf();
            }
            (None, Some(_)) => unreachable!(),
        }
    }

    pub fn send(
        &mut self,
        target_chat: GramChat,
        reply_to: Option<i64>,
        message: GramMessageContent,
    ) {
        let mut client = self.client.lock();

        if !self.chats_loaded {
            let _ = client.chats().get_chats(None, 200);
            self.chats_loaded = true;
        }

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

        client
            .messages()
            .send_message(chat_id, None, reply_to, None, None, content);
    }

    pub fn print_messages(&self, messages: &[Message]) {
        let mut builder = Builder::new();
        builder.push_record(["ID", "Sender", "Date", "Content"]);
        let max_len = self.conf.lock().settings.max_len_before_shortening;
        for msg in messages.iter().rev() {
            let sender = match &msg.sender_id {
                MessageSender::User { user_id } => shorten(max_len, &format!("user:{}", user_id)),
                MessageSender::Chat { chat_id } => shorten(max_len, &format!("chat:{}", chat_id)),
            };
            let date = chrono::DateTime::from_timestamp(msg.date as i64, 0)
                .map(|d| d.format("%H:%M:%S").to_string())
                .unwrap_or_else(|| msg.date.to_string());
            let content = match msg.content.as_ref() {
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
                _ => "[other]".to_string(),
            };
            builder.push_record([msg.id.to_string(), sender, date, content]);
        }
        let mut table = builder.build();
        Self::init_table(&mut table);
        info!(self.conf.lock().settings.color, "{}", table);
    }

    pub fn read(&mut self, target_chat: GramChat, limit: i32) {
        let mut client = self.client.lock();

        if !self.chats_loaded {
            let _ = client.chats().get_chats(None, 200);
            self.chats_loaded = true;
        }

        let chat_id = self.resolve_user(target_chat);

        if chat_id == -1 {
            return;
        }

        let history = client.chats().get_chat_history(chat_id, 0, 0, limit, false);

        info!(
            self.conf.lock().settings.color,
            "got {} messages.", history.total_count
        );
        self.print_messages(history.messages.unwrap_or_default().as_slice());
    }

    fn push_chat(
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

    pub fn chats(&mut self, limit: i32, chat_list: ChatList) {
        if limit < 1 {
			let conf = self.conf.lock();

            error!(conf.settings.color, "chat limit must be positive.");
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

        let chats = client.chats().get_chats(Some(chat_list), limit);

        self.chats_loaded = true;

        for (i, id) in chats.chat_ids.iter().enumerate() {
            let label = self
                .conf
                .lock()
                .labels
                .get(id)
                .map(|l| l.name.clone())
                .unwrap_or("".to_string());
            let chat = client.chats().get_chat(*id);
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
        info!(
            self.conf.lock().settings.color,
            "theres a total of {} chats.", chats.total_count
        );
        info!(self.conf.lock().settings.color, "{}", table);
    }

    fn init_table(table: &mut Table) {
        table
            .with(Color::BOLD)
            .with(Style::modern_rounded())
            .with(Modify::new(Rows::first()).with(Alignment::center()));
    }

    pub fn folders(&self) {
        let mut builder = Builder::new();
        builder.push_record(["ID", "Name"]);

        let folders = self.chat_folders.lock();
        let max_len = self.conf.lock().settings.max_len_before_shortening;
        for folder in folders.iter() {
            builder.push_record([
                folder.id.to_string(),
                shorten(max_len, &folder.name.text.text),
            ]);
        }

        let mut table = builder.build();
        Self::init_table(&mut table);
        info!(self.conf.lock().settings.color, "{}", table);
    }

    pub fn spawn_threads(
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

        let conf_errors = Arc::clone(&conf);

        // error thread
        std::thread::spawn(move || {
            for err in recvs.error_rx {
                error!(conf_errors.lock().settings.color, "error: {:#?}", err);
            }
        });
    }

    fn notify(
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
                let user = client.users().get_user(user_id);

                shorten(
                    max_len,
                    &conf.settings.user_sender_mode.with_user(user.clone()),
                )
            }
            MessageSender::Chat { chat_id } => {
                let chat = client.chats().get_chat(chat_id);

                shorten(
                    max_len,
                    &conf
                        .settings
                        .chat_sender_mode
                        .with_chat(conf.clone(), chat.clone()),
                )
            }
        };

        let content_str = match *message.content {
            MessageContent::MessageText { text, .. } => shorten(max_len, &text.text),
            MessageContent::MessageDocument { document, .. } => {
                shorten(max_len, &document.file_name)
            }
            MessageContent::MessageVideo { video, .. } => shorten(max_len, &video.file_name),
            MessageContent::MessagePhoto { caption, .. } => {
                shorten(max_len, &format!("<video> {}", caption.text))
            }
            _ => shorten(
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

    pub fn prompt(&self) -> String {
        if self.conf.lock().settings.color {
            cformat!("<m!,bold>gram></>")
        } else {
            "gram>".to_string()
        }
    }

    pub fn refresh_conf(&mut self) {
        let read_conf = std::fs::read_to_string(&self.conf_path).unwrap();
        *self.conf.lock() = toml::from_str(&read_conf).expect("failed to parse config");
    }

    pub fn save_conf(&self) {
        let conf = self.conf.lock();
        let serialized =
            toml::to_string_pretty(&*conf).expect("failed to serialize current gram config");

        let _ = std::fs::write(&self.conf_path, serialized);
    }
}

impl Drop for Interpreter {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_millis(30));
        self.save_conf();
    }
}
