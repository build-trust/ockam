use clap::Args;
use colorful::Colorful;
use miette::miette;

use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::authenticator::direct::Members;
use ockam_api::colors::color_primary;
use ockam_api::nodes::InMemoryNode;
use ockam_api::{fmt_err, fmt_ok};
use ockam_multiaddr::MultiAddr;

use super::{create_authority_client, get_project};
use crate::util::api::IdentityOpts;
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Add members to a Project, as an authorized enroller, directly, or via an enrollment ticket
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct DeleteCommand {
    #[command(flatten)]
    identity_opts: IdentityOpts,

    /// Which project's member to delete
    #[arg(long, short, value_name = "ROUTE_TO_PROJECT")]
    to: Option<MultiAddr>,

    #[arg(value_name = "IDENTIFIER")]
    member: Option<Identifier>,

    /// Delete all members of the project except the default identity
    #[arg(long, conflicts_with = "member")]
    all: bool,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "project-member delete".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let identity = opts
            .state
            .get_named_identity_or_default(&self.identity_opts.identity)
            .await?;

        let project = get_project(&opts.state, &self.to).await?;

        let node = InMemoryNode::start_with_project_name(
            ctx,
            &opts.state,
            Some(project.name().to_string()),
        )
        .await?;

        let authority_node_client =
            create_authority_client(&node, &opts.state, &self.identity_opts, &project).await?;

        match (&self.member, self.all) {
            (Some(member), _) => {
                authority_node_client
                    .delete_member(ctx, member.clone())
                    .await?;
                opts.terminal.stdout().plain(fmt_ok!(
                    "Identifier {} is no longer a member of the Project. It won't be able to get a credential and access Project resources, like portals of other members",
                    color_primary(member.to_string())
                ));
            }
            (None, true) => {
                if !opts
                    .state
                    .is_identity_enrolled(&Some(identity.name()))
                    .await?
                {
                    return Err(miette!(fmt_err!(
                        "You need to use an enrolled identity to delete all the members from a project."
                    )));
                }
                authority_node_client
                    .delete_all_members(ctx, identity.identifier())
                    .await?;
                opts.terminal.stdout().plain(fmt_ok!(
                    "All identifiers except {} are no longer members of the Project.",
                    color_primary(identity.identifier().to_string())
                ));
            }
            _ => unreachable!(),
        }

        Ok(())
    }
}
