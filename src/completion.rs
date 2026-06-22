use std::borrow::Cow;

use clap::CommandFactory;
use rustyline::{
    completion::{Completer, Pair},
    highlight::Highlighter,
    hint::{Hinter, HistoryHinter},
    validate::{ValidationContext, ValidationResult, Validator},
    Helper,
};

use crate::Cli;

pub struct GramHelper {
    pub hinter: HistoryHinter,
}

impl Helper for GramHelper {}

impl Default for GramHelper {
    fn default() -> Self {
        Self {
            hinter: HistoryHinter::default(),
        }
    }
}

impl Completer for GramHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let input = &line[..pos];

        let ends_with_space = input.ends_with(' ');

        let mut tokens: Vec<&str> = input.split_whitespace().collect();

        let current = if ends_with_space {
            ""
        } else {
            tokens.pop().unwrap_or("")
        };

        let start = pos.saturating_sub(current.len());

        let mut cmd = Cli::command();

        while let Some(token) = tokens.first().copied() {
            if let Some(sub) = cmd.find_subcommand(token) {
                cmd = sub.clone();
                tokens.remove(0);
            } else {
                break;
            }
        }

        let mut out = Vec::new();

        for sub in cmd.get_subcommands() {
            let name = sub.get_name();

            if name.starts_with(current) {
                out.push(Pair {
                    display: name.to_string(),
                    replacement: name.to_string(),
                });
            }

            for alias in sub.get_all_aliases() {
                if alias.starts_with(current) {
                    out.push(Pair {
                        display: alias.to_string(),
                        replacement: alias.to_string(),
                    });
                }
            }
        }

        for arg in cmd.get_arguments() {
            if let Some(long) = arg.get_long() {
                let candidate = format!("--{long}");

                if candidate.starts_with(current) {
                    out.push(Pair {
                        display: candidate.clone(),
                        replacement: candidate,
                    });
                }
            }

            if let Some(short) = arg.get_short() {
                let candidate = format!("-{short}");

                if candidate.starts_with(current) {
                    out.push(Pair {
                        display: candidate.clone(),
                        replacement: candidate,
                    });
                }
            }

            if let Some(values) = arg.get_value_parser().possible_values() {
                for value in values {
                    let value = value.get_name();

                    if value.starts_with(current) {
                        out.push(Pair {
                            display: value.to_string(),
                            replacement: value.to_string(),
                        });
                    }
                }
            }
        }

        Ok((start, out))
    }
}

impl Hinter for GramHelper {
    type Hint = String;

    fn hint(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Option<Self::Hint> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for GramHelper {
    fn highlight<'l>(
        &self,
        line: &'l str,
        _: usize,
    ) -> Cow<'l, str> {
        Cow::Borrowed(line)
    }

    fn highlight_hint<'h>(
        &self,
        hint: &'h str,
    ) -> Cow<'h, str> {
        Cow::Owned(format!("\x1b[90m{hint}\x1b[0m"))
    }
}

impl Validator for GramHelper {
    fn validate(
        &self,
        ctx: &mut ValidationContext,
    ) -> rustyline::Result<ValidationResult> {
        let input = ctx.input();

        let quote_count = input.chars().filter(|c| *c == '"').count();

        if quote_count % 2 != 0 {
            return Ok(ValidationResult::Incomplete);
        }

        Ok(ValidationResult::Valid(None))
    }
}
