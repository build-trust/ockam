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
    _ctx: Context,
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
        let identities_creation = opts
            .state
            .get_identities(vault)
            .await?
            .identities_creation();

        let public_key = identities_creation
            .identity_vault()
            .get_public_key(&cmd.key_id)
            .await
            .into_diagnostic()?;

        identities_creation
            .identity_builder()
            .with_existing_key(cmd.key_id, public_key.stype())
            .build()
            .await
            .into_diagnostic()?
    };
    let idt_name = cli_state::random_name();
    let idt_config = IdentityConfig::new(idt.identifier()).await;
    opts.state.identities.create(&idt_name, idt_config)?;
    println!("Identity attached to vault: {idt_name}");
    Ok(())
}
