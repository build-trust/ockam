use clap::Args;
use miette::miette;

use ockam::Context;
use ockam_api::cli_state;
use ockam_api::cli_state::identities::IdentityConfig;
use ockam_api::cli_state::traits::{StateDirTrait, StateItemTrait};

use ockam_identity::{IdentityChangeConstants, KeyAttributes};
use ockam_vault::SecretAttributes;

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
) -> crate::Result<()> {
    run_impl(opts, cmd).await
}

async fn run_impl(opts: CommandGlobalOpts, cmd: AttachKeyCommand) -> crate::Result<()> {
    let v_state = opts.state.vaults.get(&cmd.vault)?;
    if !v_state.config().is_aws() {
        return Err(miette!("Vault {} is not an AWS KMS vault", cmd.vault).into());
    }
    let vault = v_state.get().await?;
    let idt = {
        let attrs = SecretAttributes::NistP256;
        let key_attrs = KeyAttributes::new(IdentityChangeConstants::ROOT_LABEL.to_string(), attrs);
        opts.state
            .get_identities(vault)
            .await?
            .identities_creation()
            .create_identity_with_existing_key(&cmd.key_id, key_attrs)
            .await?
    };
    let idt_name = cli_state::random_name();
    let idt_config = IdentityConfig::new(&idt.identifier()).await;
    opts.state.identities.create(&idt_name, idt_config)?;
    println!("Identity attached to vault: {idt_name}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_core::Result;
    use ockam_identity::Identities;
    use ockam_vault::{PersistentSecretsStore, Vault};
    use ockam_vault_aws::AwsSecurityModule;
    use std::sync::Arc;

    /// This test needs to be executed with the following environment variables
    /// AWS_REGION
    /// AWS_ACCESS_KEY_ID
    /// AWS_SECRET_ACCESS_KEY
    #[tokio::test]
    #[ignore]
    async fn test_create_identity_with_external_key_id() -> Result<()> {
        let vault =
            Vault::create_with_security_module(Arc::new(AwsSecurityModule::default().await?));
        let identities = Identities::builder()
            .with_identities_vault(vault.clone())
            .build();

        // create a secret key using the AWS KMS
        let key_id = vault
            .create_persistent_secret(SecretAttributes::NistP256)
            .await?;
        let key_attrs = KeyAttributes::new(
            IdentityChangeConstants::ROOT_LABEL.to_string(),
            SecretAttributes::NistP256,
        );

        let identity = identities
            .identities_creation()
            .create_identity_with_existing_key(&key_id, key_attrs)
            .await;
        assert!(identity.is_ok());

        Ok(())
    }
}
