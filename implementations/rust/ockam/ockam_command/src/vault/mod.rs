mod default;

use crate::error::Error;
use crate::util::node_rpc;
use crate::vault::default::DefaultCommand;
use crate::{help, CommandGlobalOpts, Result};
use anyhow::anyhow;
use clap::{Args, Subcommand};
use ockam::Context;
use ockam_api::cli_state::{self, CliState, CliStateError};
use ockam_core::vault::{Secret, SecretAttributes, SecretPersistence, SecretType, SecretVault};
use ockam_identity::{Identity, IdentityStateConst, KeyAttributes};
use rand::prelude::random;

const HELP_DETAIL: &str = "";

/// Manage vaults
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    after_long_help = help::template(HELP_DETAIL)
)]
pub struct VaultCommand {
    #[command(subcommand)]
    subcommand: VaultSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum VaultSubcommand {
    /// Create a vault
    Create {
        #[arg(hide_default_value = true, default_value_t = hex::encode(&random::<[u8;4]>()))]
        name: String,

        /// Path to the Vault storage file
        #[arg(short, long)]
        path: Option<String>,

        #[arg(long, default_value = "false")]
        aws_kms: bool,
    },
    /// Attach a key to a vault
    #[command(arg_required_else_help = true)]
    AttachKey {
        /// Name of the vault to attach the key to
        vault: String,

        /// AWS KMS key to attach
        #[arg(short, long)]
        key_id: String,
    },
    /// Show vault details
    Show {
        /// Name of the vault
        name: Option<String>,
    },
    /// Delete a vault
    Delete {
        /// Name of the vault
        name: String,
    },
    /// List vaults
    List {},
    /// Set the default identity
    Default(DefaultCommand),
}

impl VaultCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self))
    }
}

async fn run_impl(ctx: Context, (opts, cmd): (CommandGlobalOpts, VaultCommand)) -> Result<()> {
    match cmd.subcommand {
        VaultSubcommand::Create { name, aws_kms, .. } => {
            let config = cli_state::VaultConfig::new(aws_kms)?;
            opts.state.vaults.create(&name, config.clone()).await?;
            println!("Vault created: {}", &name);
        }
        VaultSubcommand::AttachKey {
            vault: v_name,
            key_id,
        } => {
            let v_state = opts.state.vaults.get(&v_name)?;
            if !v_state.config.is_aws() {
                return Err(anyhow!("Vault {} is not an AWS KMS vault", v_name).into());
            }
            let v = v_state.get().await?;
            let idt = {
                let attrs =
                    SecretAttributes::new(SecretType::NistP256, SecretPersistence::Persistent, 32);
                let kid = v.secret_import(Secret::Aws(key_id), attrs).await?;
                let attrs = KeyAttributes::new(IdentityStateConst::ROOT_LABEL.to_string(), attrs);
                Identity::create_with_external_key_ext(
                    &ctx,
                    &opts.state.identities.authenticated_storage().await?,
                    &v,
                    &kid,
                    attrs,
                )
                .await?
            };
            let idt_name = cli_state::random_name();
            let idt_config = cli_state::IdentityConfig::new(&idt).await;
            opts.state.identities.create(&idt_name, idt_config)?;
            println!("Identity attached to vault: {idt_name}");
        }
        VaultSubcommand::Show { name } => {
            let name = name.unwrap_or(opts.state.vaults.default()?.name()?);
            let state = opts.state.vaults.get(&name)?;
            println!("Vault:");
            for line in state.to_string().lines() {
                println!("{:2}{}", "", line)
            }
        }
        VaultSubcommand::List {} => {
            let states = opts.state.vaults.list()?;
            if states.is_empty() {
                return Err(anyhow!("No vaults registered on this system!").into());
            }
            for (idx, vault) in states.iter().enumerate() {
                println!("Vault[{idx}]:");
                for line in vault.to_string().lines() {
                    println!("{:2}{}", "", line)
                }
            }
        }
        VaultSubcommand::Delete { name } => {
            let state = opts.state.vaults;
            // Check if exists
            return match state.get(&name) {
                // If it exists, proceed
                Ok(_) => {
                    state.delete(&name).await?;
                    println!("Vault '{name}' deleted");
                    Ok(())
                }
                // Return the appropriate error
                Err(err) => match err {
                    CliStateError::NotFound(_) => Err(anyhow!("Vault '{name}' not found").into()),
                    _ => Err(err.into()),
                },
            };
        }
        VaultSubcommand::Default(cmd) => cmd.run(opts),
    }
    Ok(())
}

pub fn default_vault_name() -> String {
    let res_cli = CliState::try_default();

    let cli_state = match res_cli {
        Ok(cli_state) => cli_state,
        Err(err) => {
            eprintln!("Error initializing command state. \n\n {err:?}");
            let command_err: Error = err.into();
            std::process::exit(command_err.code());
        }
    };

    let default_name = cli_state.vaults.default().map(|v| v.name);

    match default_name {
        Ok(name) => name,
        Err(_) => "default".to_string(),
    }
}
