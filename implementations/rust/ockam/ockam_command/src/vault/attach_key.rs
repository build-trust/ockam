use anyhow::anyhow;
use clap::Args;
use std::sync::Arc;

use ockam::Context;
use ockam_api::cli_state;

use ockam_core::vault::{Secret, SecretAttributes, SecretPersistence, SecretType, SecretVault};
use ockam_identity::{Identity, IdentityStateConst, KeyAttributes};

use crate::util::node_rpc;
use crate::CommandGlobalOpts;

#[derive(Clone, Debug, Args)]
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
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, AttachKeyCommand),
) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: AttachKeyCommand,
) -> crate::Result<()> {
    let v_state = opts.state.vaults.get(&cmd.vault)?;
    if !v_state.config.is_aws() {
        return Err(anyhow!("Vault {} is not an AWS KMS vault", cmd.vault).into());
    }
    let v = v_state.get().await?;
    let idt = {
        let attrs = SecretAttributes::new(SecretType::NistP256, SecretPersistence::Persistent, 32);
        let kid = v.secret_import(Secret::Aws(cmd.key_id), attrs).await?;
        let attrs = KeyAttributes::new(IdentityStateConst::ROOT_LABEL.to_string(), attrs);
        Identity::create_with_external_key_ext(
            ctx,
            opts.state.identities.authenticated_storage().await?,
            Arc::new(v),
            &kid,
            attrs,
        )
        .await?
    };
    let idt_name = cli_state::random_name();
    let idt_config = cli_state::IdentityConfig::new(&idt).await;
    opts.state.identities.create(&idt_name, idt_config)?;
    println!("Identity attached to vault: {idt_name}");
    Ok(())
}
