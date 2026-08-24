use crate::interp::{Interpreter, commands::AuthCommands};
use crate::util;

impl AuthCommands for Interpreter {
    fn auth(&mut self) {
        let mut client = self.client.lock();

        if let Err(err) = client.authenticate_with_console() {
            util::error!(
                self.conf.lock().settings.color,
                "authentication failed: {}",
                err
            );
            return;
        }

        let me = match client.general().get_me() {
            Ok(me) => me,
            Err(err) => {
                util::error!(
                    self.conf.lock().settings.color,
                    "telegram error {}: {}",
                    err.code,
                    err.message
                );
                return;
            }
        };

        self.me = Some(me.id);
    }

    fn unauth(&mut self) {
        let _ = std::fs::remove_dir_all("db");
    }
}
