use crate::{help, OckamCommand};
use clap::{Args, CommandFactory};
use clap_complete::{generate, Shell};
use std::io;

const HELP_DETAIL: &str = include_str!("../constants/completion/help_detail.txt");

/// Generate shell completion scripts
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true, after_long_help = help::template(HELP_DETAIL))]
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
