use crate::interp::{Interpreter, commands::AuthCommands};

impl AuthCommands for Interpreter {
    fn auth(&mut self) {
        let mut client = self.client.lock();

        client.authenticate_with_console().unwrap();

		let me = client
            .general()
            .get_me();

        self.me = Some(me.id);
    }

    fn unauth(&mut self) {
        let _ = std::fs::remove_dir_all("db");
    }
}
