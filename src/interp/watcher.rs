use notify::Event;
use notify::RecursiveMode;
use notify::Watcher;
use td_api::TdClient;

use crate::conf::GramConf;
use crate::util::*;
use crate::Interpreter;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

impl Interpreter {
	pub(super) fn watcher_thread(conf: Arc<parking_lot::Mutex<GramConf>>, conf_path: PathBuf, shutdown: Arc<AtomicBool>) {
		// conf watcher thread
		let conf_path = conf_path.clone();
		let watch_dir = conf_path.parent().unwrap().to_path_buf();
		std::thread::spawn(move || {
			let conf_watcher = Arc::clone(&conf);
			let conf_log = Arc::clone(&conf);
			let conf_path = conf_path.clone();

			let mut watcher =
				match ::notify::recommended_watcher(move |res: ::notify::Result<Event>| {
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

	pub(super) fn poll_thread(client: Arc<parking_lot::Mutex<TdClient>>, shutdown: Arc<AtomicBool>) {
		std::thread::spawn(move || {
			while !shutdown.load(Ordering::Relaxed) {
				client.lock().poll();
			}
		});
	}
}
