use clap::{arg, Args};
use ockam::Context;
use ockam_identity::{credential::Credential, IdentityIdentifier};

use crate::{
    credential::validate_encoded_cred, util::node_rpc, vault::default_vault_name, CommandGlobalOpts,
};
use anyhow::anyhow;

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
        let cred_name = cred_state.name()?;
        opts.state.credentials.get(&cred_name)?;

        let config = cred_state.config().await?;

        let bytes = match hex::decode(&config.encoded_credential) {
            Ok(b) => b,
            Err(e) => return Err(anyhow!(e).into()),
        };

        let cred: Credential = minicbor::decode(&bytes)?;
        let issuer = IdentityIdentifier::try_from(config.issuer)?;
        let is_verified = match validate_encoded_cred(
            &config.encoded_credential,
            &issuer,
            &cmd.vault,
            &opts,
            &ctx,
        )
        .await
        {
            Ok(_) => "✅",
            Err(_) => "❌",
        };

        println!("Credential: {cred_name} {is_verified}");
        println!("{cred}");
    }

    Ok(())
}
