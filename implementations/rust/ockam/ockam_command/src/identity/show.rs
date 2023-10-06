use std::fmt::Display;

use crate::identity::{get_identity_name, initialize_identity_if_default};
use crate::output::{EncodeFormat, IdentifierDisplay, Output, VerifyingPublicKeyDisplay};
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};
use clap::Args;
use miette::IntoDiagnostic;
use ockam::identity::verified_change::VerifiedChange;
use ockam::identity::{Identifier, Identity, Vault};
use ockam_api::cli_state::traits::{StateDirTrait, StateItemTrait};
use ockam_node::Context;
use serde::Serialize;
use serde_json::{json, to_string_pretty};

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show the details of an identity
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    #[arg()]
    name: Option<String>,

    /// Show the full identity history, and not just the identifier or the name
    #[arg(short, long)]
    full: bool,

    //TODO: see if it make sense to have a --encoding argument shared across commands.
    //      note the only reason this is here right now is that project.json expect the
    //      authority' identity change history to be in hex format.  This only applies
    //      for `full` (change history) identity.
    #[arg(long, value_enum, requires = "full")]
    encoding: Option<EncodeFormat>,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_identity_if_default(&opts, &self.name);
        node_rpc(Self::run_impl, (opts, self))
    }

    async fn run_impl(
        _ctx: Context,
        options: (CommandGlobalOpts, ShowCommand),
    ) -> miette::Result<()> {
        let (opts, cmd) = options;
        let name = get_identity_name(&opts.state, &cmd.name);
        let state = opts.state.identities.get(&name)?;
        let identifier = state.config().identifier();
        let (plain, json) = if cmd.full {
            let change_history = opts
                .state
                .identities
                .identities_repository()
                .await?
                .get_identity(&identifier)
                .await
                .into_diagnostic()?;

            if Some(EncodeFormat::Hex) == cmd.encoding {
                let encoded = hex::encode(change_history.export().into_diagnostic()?);
                let json = to_string_pretty(&json!({"encoded": &encoded}));
                (encoded, json)
            } else {
                let identity: ShowIdentity = Identity::import_from_change_history(
                    Some(&identifier),
                    change_history,
                    Vault::create_verifying_vault(),
                )
                .await
                .into_diagnostic()?
                .into();

                (identity.to_string(), to_string_pretty(&identity))
            }
        } else {
            let identifier_display = IdentifierDisplay(identifier);
            (
                identifier_display.to_string(),
                to_string_pretty(&json!({"identifier": &identifier_display})),
            )
        };

        opts.terminal
            .stdout()
            .plain(&plain)
            .json(json.into_diagnostic()?)
            .machine(&plain)
            .write_line()?;

        Ok(())
    }
}

#[derive(Serialize)]
struct ShowIdentity {
    identifier: Identifier,
    changes: Vec<Change>,
}

impl From<Identity> for ShowIdentity {
    fn from(value: Identity) -> Self {
        Self {
            identifier: value.identifier().to_owned(),
            changes: value.changes().iter().cloned().map(Change::from).collect(),
        }
    }
}

impl Display for ShowIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Identifier: {}", self.identifier)?;
        for (i_num, change) in self.changes.iter().enumerate() {
            writeln!(f, "  Change[{}]:", i_num)?;
            writeln!(f, "    identifier:              {}", change.identifier)?;
            writeln!(
                f,
                "    primary_public_key:      {}",
                change.primary_public_key
            )?;
            writeln!(
                f,
                "    revoke_all_purpose_keys: {}",
                change.revoke_all_purpose_keys
            )?;
        }
        Ok(())
    }
}

impl Output for ShowIdentity {
    fn output(&self) -> crate::error::Result<String> {
        Ok(self.to_string())
    }
}

#[derive(Serialize)]
struct Change {
    pub identifier: String,
    pub primary_public_key: VerifyingPublicKeyDisplay,
    pub revoke_all_purpose_keys: bool,
}

impl From<VerifiedChange> for Change {
    fn from(value: VerifiedChange) -> Self {
        Self {
            identifier: hex::encode(value.change_hash()),
            primary_public_key: VerifyingPublicKeyDisplay(value.primary_public_key().to_owned()),
            revoke_all_purpose_keys: value.data().revoke_all_purpose_keys,
        }
    }
}
