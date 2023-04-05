use clap::{arg, Args};

use ockam::Context;

use crate::{
    credential::show::display_credential, util::node_rpc, vault::default_vault_name,
    CommandGlobalOpts,
};

#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    #[arg(default_value_t = default_vault_name())]
    pub vault: String,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ListCommand),
) -> crate::Result<()> {
    let cred_states = opts.state.credentials.list()?;

    for cred_state in cred_states {
        display_credential(&opts, &ctx, &cred_state.name()?, &cmd.vault, false).await?;
    }

    Ok(())
}
