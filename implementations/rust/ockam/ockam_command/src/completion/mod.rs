use crate::{help, OckamCommand};
use clap::{Args, CommandFactory};
use clap_complete::{generate, Shell};
use std::io;

const HELP_DETAIL: &str = "\
ABOUT:
    Generate shell completion scripts for Ockam commands.

    If you’ve installed `ockam` command using a package manager, you likely
    don't need to do any additional shell configuration to gain completion support.

    If you need to set up completions manually, follow the instructions below.
    The exact configuration file locations might vary based on your system. Remember
    to restart your shell before testing whether completions are working.

```sh
    # BASH
    $ ockam completion --shell bash > /usr/share/bash-completion/completions/ockam.bash

    # ZSH
    $ ockam completion --shell zsh > /usr/local/share/zsh/site-functions/_ockam

    # FISH
    $ ockam completion --shell fish > ~/.config/fish/completions/ockam.fish
```
";

/// Generate shell completion scripts
#[derive(Clone, Debug, Args)]
#[clap(arg_required_else_help = true, help_template = help::template(HELP_DETAIL))]
pub struct CompletionCommand {
    /// The type of shell (bash, zsh, fish)
    #[clap(display_order = 900, long, short)]
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
