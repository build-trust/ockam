use crate::{OckamCommand, HELP_TEMPLATE};
use clap::{Args, CommandFactory};
use clap_complete::{generate, Shell};
use std::io;

const EXAMPLES: &str = "\
EXAMPLES

    # Generate Completions for your shell
    $ ockam completion --shell bash > /usr/share/bash-completion/completions/ockam.bash

    # Generate Completions for your shell
    $ ockam completion --shell bash > /usr/local/share/zsh/site-functions/_ockam

LEARN MORE
";

/// Create Completion Files for your desired Shell
#[derive(Clone, Debug, Args)]
#[clap(help_template = const_str::replace!(HELP_TEMPLATE, "LEARN MORE", EXAMPLES))]
pub struct CompletionCommand {
    /// Shell Type (from bash, zsh, fish)
    #[clap(display_order = 900, long, short)]
    shell: Shell,
}

impl CompletionCommand {
    pub fn run(command: CompletionCommand) {
        generate(
            command.shell,
            &mut OckamCommand::command(),
            "ockam",
            &mut io::stdout(),
        )
    }
}
