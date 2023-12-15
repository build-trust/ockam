use clap::Args;
use colorful::Colorful;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::cli_state::random_name;

use crate::terminal::OckamColor;
use crate::util::node_rpc;
use crate::{docs, fmt_log, fmt_ok, CommandGlobalOpts};

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
            let existing_vaults = opts.state.get_named_vaults().await?.len();

            let vault = match &self.vault {
                Some(vault_name) => opts.state.get_or_create_named_vault(vault_name).await?,
                None => opts.state.get_or_create_default_named_vault().await?,
            };
            let updated_vaults = opts.state.get_named_vaults().await?.len();

            // If a new vault has been created display a message
            if updated_vaults > existing_vaults {
                opts.terminal.write_line(&fmt_log!(
                    "Default vault created: {}\n",
                    vault.name().color(OckamColor::PrimaryResource.color())
                ))?;
            };

            let identity = match &self.key_id {
                Some(key_id) => {
                    opts.state
                        .create_identity_with_key_id(&self.name, &vault.name(), key_id.as_ref())
                        .await?
                }
                None => {
                    opts.state
                        .create_identity_with_name_and_vault(&self.name, &vault.name())
                        .await?
                }
            };

            *is_finished.lock().await = true;
            Ok(identity.identifier())
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
            .json(serde_json::json!({ "identifier": &identifier }))
            .write_line()?;
        Ok(identifier.clone())
    }
}
