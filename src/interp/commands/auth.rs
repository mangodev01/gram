use crate::interp::{Interpreter, commands::AuthCommands};

impl AuthCommands for Interpreter {
    fn auth(&mut self) {
        let mut client = self.client.lock();

        client.authenticate_with_console().unwrap();
    }

    fn unauth(&mut self) {
        let _ = std::fs::remove_dir_all("db");
    }
}
