use std::path::PathBuf;

use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic, WrapErr};
use ockam_api::cli_state::UseAwsKms;
use ockam_api::{fmt_info, fmt_ok};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

use crate::shared_args::TrustOpts;
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
    /// Name of the vault to create. If omitted, a random name will be generated.
    #[arg()]
    pub name: Option<String>,

    /// Path where the vault will be created.
    /// When omitted, the vault will be created in the Ockam home directory,
    /// using the name as filename.
    #[arg(long)]
    pub path: Option<PathBuf>,

    /// The route to a remote vault.
    /// The destination vault must be running the `remote_ockam_vault` service.
    #[arg(long, value_name = "ROUTE", requires = "identity")]
    pub route: Option<MultiAddr>,

    /// The identity to access the remote vault. Must be used in conjunction with `--route`.
    #[arg(long, value_name = "IDENTITY_NAME", requires = "route")]
    pub identity: Option<String>,

    /// Use AWS KMS
    #[arg(long, default_value = "false")]
    pub aws_kms: bool,

    #[command(flatten)]
    pub trust_opts: TrustOpts,
}

#[async_trait]
impl Command for CreateCommand {
    const NAME: &'static str = "vault create";

    async fn async_run(self, _ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        if self.route.is_some() && opts.state.get_named_vaults().await?.is_empty() {
            Err(miette!(
                "Cannot create a remote vault without a local vault. Create a local vault first."
            ))?;
        }

        if opts.state.get_named_vaults().await?.is_empty() {
            opts.terminal.write_line(fmt_info!(
                "This is the first vault to be created in this environment. It will be set as the default vault"
            ))?;
        }

        let vault = if let Some(route) = self.route {
            let identity = self
                .identity
                .ok_or_else(|| miette!("Identity must be provided when using --route"))?;
            let identity = opts.state.get_identifier_by_name(&identity).await?;

            let authority_options = opts
                .state
                .retrieve_authority_options(
                    &self.trust_opts.project_name,
                    &self.trust_opts.authority_identity,
                    &self.trust_opts.authority_route,
                    &self.trust_opts.credential_scope,
                )
                .await
                .into_diagnostic()
                .wrap_err("Failed to retrieve authority options, either manually specify one or enroll to a project.")?;

            opts.state
                .create_remote_vault(
                    self.name,
                    route,
                    identity,
                    authority_options.identifier,
                    authority_options.multiaddr,
                    authority_options.credential_scope,
                )
                .await?
        } else {
            opts.state
                .create_named_vault(self.name, self.path, UseAwsKms::from(self.aws_kms))
                .await?
        };

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
