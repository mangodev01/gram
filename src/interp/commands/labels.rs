use crate::{Interpreter, conf::GramLabel, interp::{SettingCommands, commands::LabelCommands}, util};

impl LabelCommands for Interpreter {
    fn label(&mut self, chat_id: i64, name: String) {
        let _ = self.conf.lock().labels.insert(chat_id, GramLabel { name });
        self.save_conf();
    }

    fn get_label(&self, chat_id: i64) {
        let conf = self.conf.lock();

        let label = conf.labels.get(&chat_id);

        if let Some(label) = label {
            util::success!(conf.settings.color, "chat has label {}", label.name);
        } else {
            util::error!(
                conf.settings.color,
                "chat doesn't have label attached. attach one by using `label {{chat}} {{label}}`"
            );
        }
    }

}
