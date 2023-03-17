use crate::{docs, OckamCommand};
use clap::{Args, CommandFactory};
use clap_complete::{generate, Shell};
use std::io;

const LONG_ABOUT: &str = include_str!("../static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("../static/after_long_help.txt");

/// Generate Shell Completion Scripts
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CompletionCommand {
    /// The type of shell (bash, zsh, fish)
    #[arg(display_order = 900, long, short)]
    shell: Shell,
}

impl CompletionCommand {
    pub fn run(self) {
        generate(
            self.shell,
            &mut OckamCommand::command(),
            "ockam",
            &mut io::stdout(),
        )
    }
}
