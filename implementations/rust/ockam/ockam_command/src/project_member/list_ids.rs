use clap::Args;

use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::authenticator::direct::Members;
use ockam_api::nodes::InMemoryNode;
use ockam_multiaddr::MultiAddr;

use super::{create_authority_client, get_project};
use crate::output::Output;
use crate::util::api::IdentityOpts;
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts, Result};

const LONG_ABOUT: &str = include_str!("./static/list_ids/long_about.txt");

/// Add members to a Project, as an authorized enroller, directly, or via an enrollment ticket
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
)]
pub struct ListIdsCommand {
    #[command(flatten)]
    identity_opts: IdentityOpts,

    /// Which project members to request
    #[arg(long, short, value_name = "ROUTE_TO_PROJECT")]
    to: Option<MultiAddr>,
}

impl ListIdsCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "project-member list-ids".into()
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

        let member_ids = authority_node_client
            .list_member_ids(ctx)
            .await?
            .into_iter()
            .map(IdentifierOutput)
            .collect();

        print_member_ids(&opts, member_ids)?;

        Ok(())
    }
}

struct IdentifierOutput(Identifier);

impl Output for IdentifierOutput {
    fn output(&self) -> Result<String> {
        Ok(self.0.to_string())
    }
}

fn print_member_ids(
    opts: &CommandGlobalOpts,
    member_ids: Vec<IdentifierOutput>,
) -> miette::Result<()> {
    let plain = opts.terminal.build_list(
        &member_ids,
        "Member Ids",
        "No members found on that Authority node.",
    )?;

    opts.terminal.clone().stdout().plain(plain).write_line()?;

    Ok(())
}
