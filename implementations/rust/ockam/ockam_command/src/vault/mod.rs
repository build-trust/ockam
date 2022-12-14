use crate::util::node_rpc;
use crate::{help, CommandGlobalOpts, Result};
use anyhow::anyhow;
use clap::{Args, Subcommand};
use ockam::Context;
use ockam_api::cli_state;
use ockam_core::vault::{Secret, SecretAttributes, SecretPersistence, SecretType, SecretVault};
use ockam_identity::{Identity, IdentityStateConst, KeyAttributes};
use rand::prelude::random;
use std::path::PathBuf;

#[derive(Clone, Debug, Args)]
#[command(hide = help::hide())]
pub struct VaultCommand {
    #[command(subcommand)]
    subcommand: VaultSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum VaultSubcommand {
    Create {
        #[arg(hide_default_value = true, default_value_t = hex::encode(&random::<[u8;4]>()))]
        name: String,

        /// Path to the Vault storage file
        #[arg(short, long)]
        path: Option<String>,

        #[arg(long, default_value = "false")]
        aws_kms: bool,
    },
    AttachKey {
        /// Name of the vault to attach the key to
        vault: String,

        /// AWS KMS key to attach
        #[arg(short, long)]
        key_id: String,
    },
}

impl VaultCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(ctx: Context, (opts, cmd): (CommandGlobalOpts, VaultCommand)) -> Result<()> {
    match cmd.subcommand {
        VaultSubcommand::Create {
            name,
            path,
            aws_kms,
        } => {
            let path = path.map(PathBuf::from).unwrap_or_else(|| {
                cli_state::VaultConfig::path(&name).expect("Failed to build Vault's path")
            });
            let config = cli_state::VaultConfig::new(path, aws_kms)?;
            opts.state.vaults.create(&name, config.clone()).await?;
            println!("Vault created: {}", &name);
        }
        VaultSubcommand::AttachKey {
            vault: v_name,
            key_id,
        } => {
            let v_config = opts.state.vaults.get(&v_name)?.config;
            if !v_config.is_aws() {
                return Err(anyhow!("Vault {} is not an AWS KMS vault", v_name).into());
            }
            let v = v_config.get().await?;
            let idt = {
                let attrs =
                    SecretAttributes::new(SecretType::NistP256, SecretPersistence::Persistent, 32);
                let kid = v.secret_import(Secret::Aws(key_id), attrs).await?;
                let attrs = KeyAttributes::new(IdentityStateConst::ROOT_LABEL.to_string(), attrs);
                Identity::create_ext(&ctx, &v, &kid, attrs).await?
            };
            let idt_name = cli_state::random_name();
            let idt_config = cli_state::IdentityConfig::new(&idt).await;
            opts.state.identities.create(&idt_name, idt_config)?;
            println!("Identity attached to vault: {}", idt_name);
        }
    }
    Ok(())
}
