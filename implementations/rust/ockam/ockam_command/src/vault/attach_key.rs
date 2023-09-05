use clap::Args;
use miette::{miette, IntoDiagnostic};

use ockam::Context;
use ockam_api::cli_state;
use ockam_api::cli_state::identities::IdentityConfig;
use ockam_api::cli_state::traits::{StateDirTrait, StateItemTrait};

use crate::util::node_rpc;
use crate::CommandGlobalOpts;

/// Attach a key to a vault
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true)]
pub struct AttachKeyCommand {
    /// Name of the vault to attach the key to
    vault: String,

    /// AWS KMS key to attach
    #[arg(short, long)]
    key_id: String,
}

impl AttachKeyCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(
    mut _ctx: Context,
    (opts, cmd): (CommandGlobalOpts, AttachKeyCommand),
) -> miette::Result<()> {
    run_impl(opts, cmd).await
}

async fn run_impl(opts: CommandGlobalOpts, cmd: AttachKeyCommand) -> miette::Result<()> {
    let v_state = opts.state.vaults.get(&cmd.vault)?;
    if !v_state.config().is_aws() {
        return Err(miette!("Vault {} is not an AWS KMS vault", cmd.vault));
    }
    let vault = v_state.get().await?;
    let idt = {
        let builder = opts
            .state
            .get_identities(vault)
            .await?
            .identities_creation()
            .identity_builder();
        let builder = builder.with_existing_key(cmd.key_id);
        builder.build().await.into_diagnostic()?
    };
    let idt_name = cli_state::random_name();
    let idt_config = IdentityConfig::new(idt.identifier()).await;
    opts.state.identities.create(&idt_name, idt_config)?;
    println!("Identity attached to vault: {idt_name}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use ockam::identity::Identities;
    use ockam_core::Result;
    use ockam_vault::{SecretAttributes, Vault};
    use ockam_vault_aws::AwsSigningVault;
    use std::sync::Arc;

    /// This test needs to be executed with the following environment variables
    /// AWS_REGION
    /// AWS_ACCESS_KEY_ID
    /// AWS_SECRET_ACCESS_KEY
    /// or credentials in ~/.aws/credentials
    #[tokio::test]
    #[ignore]
    async fn test_create_identity_with_external_key_id() -> Result<()> {
        let mut vault = Vault::create();
        vault.signing_vault = Arc::new(AwsSigningVault::create().await?);
        let identities = Identities::builder().with_vault(vault.clone()).build();

        // create a secret key using the AWS KMS
        let key_id = vault
            .signing_vault
            .generate_key(SecretAttributes::NistP256)
            .await?;

        let identity = identities
            .identities_creation()
            .identity_builder()
            .with_existing_key(key_id.clone())
            .build()
            .await?;

        identities
            .identities_creation()
            .import(Some(identity.identifier()), &identity.export()?)
            .await?;

        vault.signing_vault.delete_key(key_id).await?;

        Ok(())
    }
}
