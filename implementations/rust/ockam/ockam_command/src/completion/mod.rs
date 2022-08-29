use crate::{help, OckamCommand};
use clap::{Args, CommandFactory};
use clap_complete::{generate, Shell};
use std::io;

const HELP_DETAIL: &str = "\
EXAMPLES:

```sh
    # Generate Completions for your shell
    $ ockam completion --shell bash > /usr/share/bash-completion/completions/ockam.bash

    # Generate Completions for your shell
    $ ockam completion --shell bash > /usr/local/share/zsh/site-functions/_ockam
```
";

/// Generate completion scripts for your shell
#[derive(Clone, Debug, Args)]
#[clap(help_template = help::template(HELP_DETAIL))]
pub struct CompletionCommand {
    /// Shell Type (from bash, zsh, fish)
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
