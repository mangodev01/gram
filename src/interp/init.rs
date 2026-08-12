use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use directories::ProjectDirs;
use td_api::{*, params::*};

use crate::{Interpreter, conf::{ChatFoldersCache, GramConf}, interp::SettingCommands};

const DEFAULT_CONFIG: &str = include_str!("../../default_conf.toml");

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

		// spin up config watcher thread
		Self::watcher_thread(Arc::clone(&conf), conf_path.clone(), Arc::clone(&shutdown));

		// spin up poll thread to (hopefully) safely shutdown gram
		Self::poll_thread(Arc::clone(&client), Arc::clone(&shutdown));

        Self {
            client,
            // nesting: Nesting::None,
            chat_folders,
            conf,
            conf_path,
            should_exit: false,
            chats_loaded: false,
			me: None,
            shutdown,
        }
    }
}

impl Drop for Interpreter {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_millis(30));
        self.save_conf();
    }
}
