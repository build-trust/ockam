use async_trait::async_trait;
use clap::Args;

use ockam::identity::{AttributesEntry, Identifier};
use ockam::Context;
use ockam_api::authenticator::direct::Members;
use ockam_api::nodes::InMemoryNode;
use ockam_multiaddr::MultiAddr;

use crate::util::api::IdentityOpts;
use crate::{docs, Command, CommandGlobalOpts, Result};
use ockam_api::output::Output;

use super::{create_authority_client, get_project};

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");

/// List members of a Project
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
)]
pub struct ListCommand {
    #[command(flatten)]
    identity_opts: IdentityOpts,

    /// Which project members to request
    #[arg(long, short, value_name = "ROUTE_TO_PROJECT")]
    to: Option<MultiAddr>,
}

#[async_trait]
impl Command for ListCommand {
    const NAME: &'static str = "project-member list";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let project = get_project(&opts.state, &self.to).await?;

        let node = InMemoryNode::start_with_project_name(
            ctx,
            &opts.state,
            Some(project.name().to_string()),
        )
        .await?;

        let authority_node_client =
            create_authority_client(&node, &opts.state, &self.identity_opts, &project).await?;

        let members = authority_node_client
            .list_members(ctx)
            .await?
            .into_iter()
            .map(|i| MemberOutput(i.0, i.1))
            .collect();

        print_members(&opts, members)?;

        Ok(())
    }
}

struct MemberOutput(Identifier, AttributesEntry);

impl Output for MemberOutput {
    fn item(&self) -> ockam_api::Result<String> {
        Ok(format!("{}: {}", self.0, self.1))
    }
}

fn print_members(opts: &CommandGlobalOpts, member_ids: Vec<MemberOutput>) -> miette::Result<()> {
    let plain = opts.terminal.build_list(
        &member_ids,
        "Members",
        "No members found on that Authority node.",
    )?;

    opts.terminal.clone().stdout().plain(plain).write_line()?;

    Ok(())
}
