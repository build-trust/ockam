use async_trait::async_trait;
use clap::Args;
use serde::Serialize;
use std::sync::Arc;

use crate::node_command::InMemoryNodeCommand;
use crate::shared_args::IdentityOpts;
use crate::{docs, Command, CommandGlobalOpts, Result};
use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::authenticator::direct::Members;
use ockam_api::nodes::InMemoryNode;
use ockam_api::output::Output;

const LONG_ABOUT: &str = include_str!("./static/list_ids/long_about.txt");

/// List members ID's of a Project
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
)]
pub struct ListIdsCommand {
    #[command(flatten)]
    identity_opts: IdentityOpts,

    /// The Project to list members from
    #[arg(long, short, value_name = "PROJECT_NAME")]
    project_name: Option<String>,
}

#[derive(Clone)]
struct ListIdsNodeCommand {
    opts: CommandGlobalOpts,
    command: ListIdsCommand,
}

impl ListIdsNodeCommand {
    pub fn new(opts: CommandGlobalOpts, command: ListIdsCommand) -> Self {
        Self { opts, command }
    }
}

#[async_trait]
impl InMemoryNodeCommand for ListIdsNodeCommand {
    fn project_name(&self) -> Option<String> {
        self.command.project_name.clone()
    }

    fn identity_name(&self) -> Option<String> {
        self.command.identity_opts.identity_name.clone()
    }

    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let authority_node_client = self.authority_client(node.clone()).await?;

        let member_ids = authority_node_client
            .list_member_ids(node.ctx())
            .await?
            .into_iter()
            .map(|identifier| ListIdsOutput { identifier })
            .collect::<Vec<_>>();

        let plain = self
            .opts
            .terminal
            .build_list(&member_ids, "No members found on the Authority node")?;
        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(plain)
            .json_obj(&member_ids)?
            .write_line()?;

        Ok(())
    }
}

#[async_trait]
impl Command for ListIdsCommand {
    const NAME: &'static str = "project-member list-ids";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        ListIdsNodeCommand::new(opts.clone(), self.clone())
            .execute(ctx, opts.state)
            .await
    }
}

#[derive(Serialize)]
struct ListIdsOutput {
    identifier: Identifier,
}

impl Output for ListIdsOutput {
    fn item(&self) -> ockam_api::Result<String> {
        Ok(format!("{}", self.identifier))
    }
}
