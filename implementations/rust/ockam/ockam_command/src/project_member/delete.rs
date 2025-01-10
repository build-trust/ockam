use super::authority_client;
use crate::shared_args::IdentityOpts;
use crate::{docs, Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::authenticator::direct::Members;
use ockam_api::colors::color_primary;
use ockam_api::{fmt_info, fmt_ok};
use ockam_core::TryClone;
use serde::Serialize;
use std::fmt::Display;
use std::time::Duration;
use tokio::task::JoinSet;
use tokio::time::sleep;
use tracing::warn;

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Remove members from a Project
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct DeleteCommand {
    #[command(flatten)]
    identity_opts: IdentityOpts,

    /// The Project that the member belongs to
    #[arg(long, short, value_name = "PROJECT_NAME")]
    project_name: Option<String>,

    /// The Identifier of the member to delete
    #[arg(value_name = "IDENTIFIER")]
    member: Option<Identifier>,

    /// Delete all members of the Project except the current default Identity
    #[arg(long, conflicts_with = "member")]
    all: bool,
}

#[async_trait]
impl Command for DeleteCommand {
    const NAME: &'static str = "project-member delete";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        if self.member.is_none() && !self.all {
            return Err(miette!(
                "You need to specify either an identifier to delete or use the --all flag to delete all the members from a project."
            ));
        }

        let (authority_node_client, project_name) =
            authority_client(ctx, &opts, &self.identity_opts, &self.project_name).await?;

        let identity = opts
            .state
            .get_named_identity_or_default(&self.identity_opts.identity_name)
            .await?;

        let mut output = DeleteMemberOutput {
            project: project_name.clone(),
            identifiers: vec![],
        };

        // Delete the passed member
        if let Some(member) = &self.member {
            authority_node_client
                .delete_member(ctx, member.clone())
                .await?;
            output.identifiers.push(member.clone());
        }
        // Try to delete all members except the current default identity
        else if self.all {
            if !opts
                .state
                .is_identity_enrolled(&Some(identity.name()))
                .await?
            {
                return Err(miette!(
                        "You need to use an enrolled identity to delete all the members from a Project."
                    ));
            }
            let self_identifier = identity.identifier();
            let member_identifiers = authority_node_client.list_member_ids(ctx).await?;
            if !member_identifiers.is_empty() {
                opts.terminal.write_line(fmt_info!(
                    "Found {} members in the Project {}",
                    member_identifiers.len(),
                    project_name
                ))?;
            }

            let members_to_delete = member_identifiers
                .into_iter()
                .filter(|id| id != &self_identifier)
                .collect::<Vec<_>>();

            let pb = opts.terminal.spinner();
            if let Some(pb) = &pb {
                pb.set_message("Deleting members...");
            }
            for chunk in members_to_delete.chunks(10).map(|c| c.to_vec()) {
                let mut set: JoinSet<Option<Identifier>> = JoinSet::new();
                for identifier in chunk {
                    let authority_node_client = authority_node_client.clone();
                    let ctx = ctx.try_clone()?;
                    set.spawn(async move {
                        sleep(tokio_retry::strategy::jitter(Duration::from_millis(500))).await;
                        if let Err(e) = authority_node_client
                            .delete_member(&ctx, identifier.clone())
                            .await
                        {
                            warn!("Failed to delete member {identifier}: {e}",);
                            None
                        } else {
                            Some(identifier)
                        }
                    });
                }
                while let Some(res) = set.join_next().await {
                    if let Some(identifier) = res.into_diagnostic()? {
                        output.identifiers.push(identifier);
                        if let Some(pb) = &pb {
                            pb.inc(1);
                        }
                    }
                }
            }
        } else {
            unreachable!("Either a member or the --all flag should be set");
        }

        opts.terminal
            .stdout()
            .plain(output.to_string())
            .json_obj(&output)?
            .write_line()?;

        Ok(())
    }
}

#[derive(Serialize)]
struct DeleteMemberOutput {
    project: String,
    identifiers: Vec<Identifier>,
}

impl Display for DeleteMemberOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.identifiers[..] {
            [] => {
                writeln!(
                    f,
                    "{}",
                    fmt_ok!(
                        "There are no members that can be deleted from the Project {}",
                        self.project
                    )
                )?;
            }
            [identifier] => {
                writeln!(
                    f,
                    "{}",
                    fmt_ok!(
                        "Identifier {} is no longer a member of the Project. \
                        It won't be able to get a credential and access Project resources, \
                        like portals of other members",
                        color_primary(identifier.to_string())
                    )
                )?;
            }
            _ => {
                writeln!(
                    f,
                    "{}",
                    fmt_ok!(
                        "{} members were deleted from the Project {}",
                        &self.identifiers.len(),
                        self.project
                    )
                )?;
            }
        }
        Ok(())
    }
}
