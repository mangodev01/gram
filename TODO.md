- [x] working parser && repl implementation
- [ ] make encryption configurable
- [ ] handle td errors correctly (tell rustyline to redraw)
- [ ] calls (unsure if needed; deemed way too complicated atm)

## settings
(types prefixed with ? here mean that they're optional)
settings are in `[settings]` in conf.toml

| Priority | Status | Key                       | Type | Meaning                                                                                                                   | Value |
|----------|--------| ---                       | ---- | -------                                                                                                                   | ----- |
| P1       | [x]    | user_sender_mode          | enum | how sender users are labeled in notifications - by first name & last name, by first name, or by last name                 | flnam |
| P1       | [x]    | chat_sender_mode          | enum | how sender chats are labeled in notifications - by label, by title or by chat id                                          | title |
| P1       | [x]    | max_len_before_shortening | enum | max length before shortening (appending '...') message / username / other text, -1 means don't shorten at all             |  -1   |
| P2       | [x]    | dev_mode                  | bool | show/hide telegram internal IDs. true=show, false=hide                                                                    | true  |
| P2       | [x]    | color                     | bool | enable/disable color in the CLI                                                                                           | true  |
| P1       | [ ]    | encryption_key            | ?str | enable encryption with the set encryption key. NOT setting this means telegram traffic is unencrypted                     | false |

## refactors
- [x] split `interp.rs` into modules — `display.rs` (print_messages, push_chat, init_table), `notify.rs` (notify thread), `watcher.rs` (conf watcher thread), keep `interp.rs` thin (dispatch + state)

## features
- [x] REPL completion with clap
