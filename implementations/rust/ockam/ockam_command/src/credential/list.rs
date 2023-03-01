use clap::{arg, Args};
use colorful::Colorful;
use ockam::Context;
use ockam_identity::IdentityIdentifier;

use crate::{
    credential::validate_encoded_cred, util::node_rpc, vault::default_vault_name, CommandGlobalOpts,
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
        let cred_name = cred_state.name()?;

        let cred_config = opts.state.credentials.get(&cred_name)?.config().await?;

        let issuer = IdentityIdentifier::try_from(cred_config.issuer.to_string())?;
        let is_verified = match validate_encoded_cred(
            &cred_config.encoded_credential,
            &issuer,
            &cmd.vault,
            &opts,
            &ctx,
        )
        .await
        {
            Ok(_) => format!("Verified({})", "✔︎".light_green()),
            Err(_) => format!("Unverified({})", "✕".light_red()),
        };

        let cred = cred_config.credential()?;
        println!("Credential: {cred_name} {is_verified}");
        println!("{cred}");
    }

    Ok(())
}
