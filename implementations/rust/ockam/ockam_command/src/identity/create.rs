use crate::terminal::OckamColor;
use crate::util::node_rpc;
use crate::{docs, fmt_log, fmt_ok, CommandGlobalOpts};
use clap::Args;
use colorful::Colorful;
use miette::miette;
use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::cli_state::random_name;
use ockam_api::cli_state::traits::{StateDirTrait, StateItemTrait};
use ockam_vault::{HandleToSecret, SigningSecretKeyHandle};
use tokio::sync::Mutex;
use tokio::try_join;

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create a new identity
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    #[arg(hide_default_value = true, default_value_t = random_name())]
    name: String,

    /// Vault name to store the identity key
    #[arg(long, value_name = "VAULT_NAME", global = true)]
    vault: Option<String>,

    /// Key ID to use for the identity creation
    #[arg(short, long)]
    key_id: Option<String>,
}

impl CreateCommand {
    pub fn new(name: String, vault: Option<String>, key_id: Option<String>) -> CreateCommand {
        CreateCommand {
            name,
            vault,
            key_id,
        }
    }

    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(Self::run_impl, (options, self))
    }

    async fn run_impl(
        _ctx: Context,
        (options, cmd): (CommandGlobalOpts, CreateCommand),
    ) -> miette::Result<()> {
        cmd.create_identity(options).await.map(|_| ())
    }

    pub async fn create_identity(&self, opts: CommandGlobalOpts) -> miette::Result<Identifier> {
        opts.terminal.write_line(&fmt_log!(
            "Creating identity {}...\n",
            &self
                .name
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ))?;

        let is_finished: Mutex<bool> = Mutex::new(false);

        let send_req = async {
            let default_vault_created =
                self.vault.is_none() && opts.state.vaults.default().is_err();
            let vault_state = opts.state.create_vault_state(self.vault.as_deref()).await?;
            if default_vault_created {
                opts.terminal.write_line(&fmt_log!(
                    "Default vault created: {}\n",
                    &vault_state
                        .name()
                        .to_string()
                        .color(OckamColor::PrimaryResource.color())
                ))?;
            }

            let vault = vault_state.get().await?;

            let identities_creation = opts
                .state
                .get_identities(vault)
                .await?
                .identities_creation();

            // Create an identity using the KMS key, if provided.
            let identity = match &self.key_id {
                Some(key_id) => {
                    if !vault_state.config().is_aws() {
                        Err(miette!(
                            "Vault {} is not an AWS KMS vault",
                            self.vault.clone().unwrap_or("default".to_string()),
                        ))
                    } else {
                        let handle = SigningSecretKeyHandle::ECDSASHA256CurveP256(
                            HandleToSecret::new(key_id.as_bytes().to_vec()),
                        );

                        Ok(identities_creation
                            .identity_builder()
                            .with_existing_key(handle)
                            .build()
                            .await?)
                    }
                }
                None => Ok(identities_creation.create_identity().await?),
            }?;

            opts.state
                .create_identity_state(identity.identifier(), Some(&self.name))
                .await?;

            let identifier = identity.identifier().clone();

            *is_finished.lock().await = true;
            Ok(identifier)
        };

        let output_messages = vec![format!("Creating identity...")];

        let progress_output = opts
            .terminal
            .progress_output(&output_messages, &is_finished);

        let (identifier, _) = try_join!(send_req, progress_output)?;

        opts.terminal
            .stdout()
            .plain(
                fmt_ok!(
                    "Identity {} \n",
                    identifier
                        .to_string()
                        .color(OckamColor::PrimaryResource.color())
                ) + &fmt_log!(
                    "created successfully as {}",
                    &self
                        .name
                        .to_string()
                        .color(OckamColor::PrimaryResource.color())
                ),
            )
            .machine(identifier.clone())
            .json(serde_json::json!({ "identity": { "identifier": &identifier } }))
            .write_line()?;
        Ok(identifier.clone())
    }
}
