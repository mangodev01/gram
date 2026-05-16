- [x] working parser && repl implementation
- [ ] make encryption configurable

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
- [ ] replace `Arc<Mutex<GramConf>>` with `arc_swap::ArcSwap<GramConf>` — eliminates all `.lock()` noise in read-heavy paths (notify, watcher, error thread), no deadlock risk, no lock ordering headaches
- [ ] enforce lock ordering at the type level — add a method that locks client → conf in the correct order and passes `(&mut TdClient, &mut GramConf)` to a closure; makes the ordering impossible to get wrong
- [ ] split `interp.rs` into modules — `display.rs` (print_messages, push_chat, init_table), `notify.rs` (notify thread), `watcher.rs` (conf watcher thread), keep `interp.rs` thin (dispatch + state)
- [ ] replace `settings!` proc-macro with a hand-written `GramSettings` struct — reduces build complexity, no macro magic for ~40 lines of boilerplate
- [ ] make `info!`/`error!`/`success!` methods on `Interpreter` instead of macros requiring `conf.settings.color` everywhere
- [ ] avoid `std::thread::spawn` per notification — reuse a thread pool or call `notify_rust` synchronously (it's fast enough)
- [ ] wire up TDLib error feedback into command results — `client.chats().get_chats(...)` returns silently even on failure; correlate error channel messages with request IDs or use synchronous execute where available
