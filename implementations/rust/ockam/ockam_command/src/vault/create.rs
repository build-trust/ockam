use std::path::PathBuf;

use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use ockam_api::cli_state::UseAwsKms;
use ockam_api::{fmt_info, fmt_ok};

use ockam_node::Context;

use crate::{docs, Command, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a vault
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    #[arg()]
    pub name: Option<String>,

    #[arg(long)]
    pub path: Option<PathBuf>,

    #[arg(long, default_value = "false")]
    pub aws_kms: bool,
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "vault create";

    async fn run(self, _ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        if opts.state.get_named_vaults().await?.is_empty() {
            opts.terminal.write_line(fmt_info!(
            "This is the first vault to be created in this environment. It will be set as the default vault"
        ))?;
        }

        let vault = opts
            .state
            .create_named_vault(self.name, self.path, UseAwsKms::from(self.aws_kms))
            .await?;

        opts.terminal
            .stdout()
            .plain(fmt_ok!("Vault created with name '{}'!", vault.name()))
            .machine(vault.name())
            .json(serde_json::json!({ "name": &vault.name() }))
            .write_line()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::run::parser::resource::utils::parse_cmd_from_args;

    use super::*;

    #[test]
    fn command_can_be_parsed_from_name() {
        let cmd = parse_cmd_from_args(CreateCommand::NAME, &[]);
        assert!(cmd.is_ok());
    }
}
