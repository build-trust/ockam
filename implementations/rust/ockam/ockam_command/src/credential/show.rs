use clap::{arg, Args};
use colorful::Colorful;
use ockam::Context;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};

use crate::{
    credential::validate_encoded_cred, util::node_rpc, vault::default_vault_name, CommandGlobalOpts,
};

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    #[arg()]
    pub credential_name: String,

    #[arg()]
    pub vault: Option<String>,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> crate::Result<()> {
    let vault_name = cmd
        .vault
        .clone()
        .unwrap_or_else(|| default_vault_name(&opts.state));
    display_credential(&opts, &cmd.credential_name, &vault_name).await
}

pub(crate) async fn display_credential(
    opts: &CommandGlobalOpts,
    cred_name: &str,
    vault_name: &str,
) -> crate::Result<()> {
    let cred = opts.state.credentials.get(cred_name)?;
    let cred_config = cred.config();

    let is_verified = match validate_encoded_cred(
        &cred_config.encoded_credential,
        &cred_config.issuer.identifier(),
        vault_name,
        opts,
    )
    .await
    {
        Ok(_) => "✔︎".light_green(),
        Err(_) => "✕".light_red(),
    };

    let cred = cred_config.credential()?;
    println!("Credential: {cred_name} {is_verified}");
    println!("{cred}");

    Ok(())
}
