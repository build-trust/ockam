use clap::{arg, Args};

use colorful::Colorful;
use ockam::Context;
use ockam_api::cli_state::StateDirTrait;

use crate::{
    fmt_log, terminal::OckamColor, util::node_rpc, vault::default_vault_name, CommandGlobalOpts,
};

use super::CredentialOutput;

#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    /// Name of the Vault from which to retrieve the credentials
    #[arg(value_name = "VAULT_NAME")]
    pub vault: Option<String>,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ListCommand),
) -> miette::Result<()> {
    opts.terminal
        .write_line(&fmt_log!("Listing Credentials...\n"))?;

    let vault_name = cmd
        .vault
        .clone()
        .unwrap_or_else(|| default_vault_name(&opts.state));
    let mut credentials: Vec<CredentialOutput> = Vec::new();

    for cred_state in opts.state.credentials.list()? {
        let cred = CredentialOutput::try_from_state(&opts, &cred_state, &vault_name).await?;
        credentials.push(cred);
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
