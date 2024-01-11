use clap::{arg, Args};

use colorful::Colorful;
use ockam::Context;

use crate::{fmt_log, terminal::OckamColor, util::node_rpc, CommandGlobalOpts};

use super::CredentialOutput;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    /// Name of the Vault from which to retrieve the credentials
    #[arg(value_name = "VAULT_NAME")]
    pub vault: Option<String>,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(opts.rt.clone(), run_impl, (opts, self));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ListCommand),
) -> miette::Result<()> {
    opts.terminal
        .write_line(&fmt_log!("Listing Credentials...\n"))?;

    let vault_name = opts
        .state
        .get_named_vault_or_default(&cmd.vault)
        .await?
        .name();
    let mut credentials: Vec<CredentialOutput> = Vec::new();

    for credential in opts.state.get_credentials().await? {
        let credential_output = CredentialOutput::new(credential).await;
        credentials.push(credential_output);
    }

    let list = opts.terminal.build_list(
        &credentials,
        "Credentials",
        &format!(
            "No Credentials found for vault: {}",
            vault_name.color(OckamColor::PrimaryResource.color())
        ),
    )?;

    opts.terminal.stdout().plain(list).write_line()?;

    Ok(())
}
