use std::fmt::Display;

use clap::Args;
use miette::IntoDiagnostic;
use serde::Serialize;
use serde_json::{json, to_string_pretty};

use ockam::identity::verified_change::VerifiedChange;
use ockam::identity::{Identifier, Identity};
use ockam_api::cli_state::NamedIdentity;
use ockam_api::output::{EncodeFormat, Output};

use crate::identity::list::IdentityListOutput;
use crate::output::{IdentifierDisplay, VerifyingPublicKeyDisplay};
use crate::{docs, CommandGlobalOpts};

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
    pub fn name(&self) -> String {
        "identity show".into()
    }

    pub async fn run(&self, opts: CommandGlobalOpts) -> miette::Result<()> {
        if self.name.is_some() || !opts.terminal.can_ask_for_user_input() {
            ShowCommand::show_single_identity(&opts, &self.name, self.full, self.encoding.clone())
                .await?;
            return Ok(());
        }

        let identities: Vec<NamedIdentity> = opts.state.get_named_identities().await?;
        let identities_names: Vec<String> = identities.iter().map(|i| i.name()).collect();
        match identities_names.len() {
            0 => {
                opts.terminal
                    .stdout()
                    .plain("There are no identities to show")
                    .write_line()?;
            }
            1 => {
                ShowCommand::show_single_identity(
                    &opts,
                    &identities_names.first().cloned(),
                    self.full,
                    self.encoding.clone(),
                )
                .await?;
            }
            _ => {
                let selected_names = opts.terminal.select_multiple(
                    "Select one or more identities that you want to show".to_string(),
                    identities_names,
                );

                if selected_names.is_empty() {
                    opts.terminal
                        .stdout()
                        .plain("No identities selected")
                        .write_line()?;
                    return Ok(());
                }

                if opts.terminal.confirm_interactively(format!(
                    "Would you like to show these items : {:?}?",
                    selected_names
                )) {
                    ShowCommand::show_identity_list(&opts, selected_names).await?;
                }
            }
        }

        Ok(())
    }
}

impl ShowCommand {
    async fn show_single_identity(
        opts: &CommandGlobalOpts,
        name: &Option<String>,
        full: bool,
        encoding: Option<EncodeFormat>,
    ) -> miette::Result<()> {
        let identity = opts.state.get_identity_by_optional_name(name).await?;

        let (plain, json) = if full {
            if Some(EncodeFormat::Hex) == encoding {
                let change_history = identity.change_history();
                let encoded = change_history.export_as_string().into_diagnostic()?;
                let json = to_string_pretty(&json!({"encoded": &encoded}));
                (encoded, json)
            } else {
                let identity: ShowIdentity = identity.into();
                (identity.to_string(), to_string_pretty(&identity))
            }
        } else {
            let identifier_display = IdentifierDisplay(identity.identifier().clone());
            (
                identifier_display.to_string(),
                to_string_pretty(&json!({"identifier": &identifier_display})),
            )
        };

        opts.terminal
            .clone()
            .stdout()
            .plain(&plain)
            .json(json.into_diagnostic()?)
            .machine(&plain)
            .write_line()?;
        Ok(())
    }

    async fn show_identity_list(
        opts: &CommandGlobalOpts,
        selected_names: Vec<String>,
    ) -> miette::Result<()> {
        let mut identities: Vec<IdentityListOutput> = Vec::new();

        for name in selected_names {
            let identity = opts.state.get_named_identity(&name).await?;
            let identity_list_output = IdentityListOutput::new(
                identity.name(),
                identity.identifier().to_string(),
                identity.is_default(),
            );
            identities.push(identity_list_output);
        }

        let list = opts
            .terminal
            .build_list(&identities, "No identities found on this system.")?;

        opts.terminal
            .clone()
            .stdout()
            .plain(list)
            .json(json!(&identities))
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
    fn item(&self) -> ockam_api::Result<String> {
        Ok(self.padded_display())
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
