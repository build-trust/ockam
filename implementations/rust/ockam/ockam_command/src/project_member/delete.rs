use clap::Args;
use colorful::Colorful;

use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::authenticator::direct::Members;
use ockam_api::nodes::InMemoryNode;
use ockam_multiaddr::MultiAddr;

use super::{create_authority_client, get_project};
use crate::util::api::IdentityOpts;
use crate::util::async_cmd;
use crate::{docs, fmt_ok, CommandGlobalOpts};

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
    member: Identifier,
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
        let project = get_project(&opts.state, &self.to).await?;

        let node = InMemoryNode::start_with_project_name(
            ctx,
            &opts.state,
            Some(project.name().to_string()),
        )
        .await?;

        let authority_node_client =
            create_authority_client(&node, &opts.state, &self.identity_opts, &project).await?;

        authority_node_client
            .delete_member(ctx, self.member.clone())
            .await?;

        opts.terminal.stdout().plain(fmt_ok!(
            "Identifier {} is no longer a member of the Project. It won't be able to get a credential and access Project resources, like portals of other members",
            self.member
        ));

        Ok(())
    }
}
