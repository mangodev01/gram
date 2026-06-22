use tabled::builder::Builder;

use crate::{Interpreter, interp::commands::SettingCommands, util};

impl SettingCommands for Interpreter {
    fn settings(&mut self, key: Option<String>, value: Option<String>) {
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
                util::info!(conf.settings.color, "{}", table);
            }
            (Some(k), None) => {
                // get
                let conf = self.conf.lock();

                let v = match conf.settings.get(&k) {
                    Ok(v) => v,
                    Err(err) => {
                        util::error!(
                            conf.settings.color,
                            "util::error while getting setting {k}: {err}"
                        );
                        return;
                    }
                };

                util::info!(conf.settings.color, "{k}={v}");
            }
            (Some(k), Some(v)) => {
                // set
                let mut conf = self.conf.lock();

                if let Err(err) = conf.settings.set(&k, &v) {
                    util::error!(
                        conf.settings.color,
                        "util::error while setting setting {k} to \"{v}\": {err}"
                    );
                    return;
                }

                util::info!(conf.settings.color, "{k}={v}");

                drop(conf);
                self.save_conf();

                self.refresh_conf();
            }
            (None, Some(_)) => unreachable!(),
        }
    }

    fn refresh_conf(&mut self) {
        let read_conf = std::fs::read_to_string(&self.conf_path).unwrap();
        *self.conf.lock() = toml::from_str(&read_conf).expect("failed to parse config");
    }

    fn save_conf(&self) {
        let conf = self.conf.lock();
        let serialized =
            toml::to_string_pretty(&*conf).expect("failed to serialize current gram config");

        let _ = std::fs::write(&self.conf_path, serialized);
    }
}
