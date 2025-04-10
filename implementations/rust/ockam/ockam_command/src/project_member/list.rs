use async_trait::async_trait;
use clap::Args;
use std::sync::Arc;

use super::MemberOutput;
use crate::node_command::InMemoryNodeCommand;
use crate::shared_args::IdentityOpts;
use crate::{docs, Command, CommandGlobalOpts, Result};
use ockam::Context;
use ockam_api::authenticator::direct::{
    Members, OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE, OCKAM_ROLE_ATTRIBUTE_KEY,
};
use ockam_api::nodes::InMemoryNode;

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");

/// List members of a Project
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
)]
pub struct ListCommand {
    #[command(flatten)]
    identity_opts: IdentityOpts,

    /// The Project to list members from
    #[arg(long, short, value_name = "PROJECT_NAME")]
    project_name: Option<String>,

    /// Return only the enroller members
    #[arg(long, visible_alias = "enroller")]
    enrollers: bool,
}

#[derive(Clone)]
struct ListNodeCommand {
    opts: CommandGlobalOpts,
    command: ListCommand,
}

impl ListNodeCommand {
    pub fn new(opts: CommandGlobalOpts, command: ListCommand) -> Self {
        Self { opts, command }
    }
}

#[async_trait]
impl InMemoryNodeCommand for ListNodeCommand {
    fn project_name(&self) -> Option<String> {
        self.command.project_name.clone()
    }

    fn identity_name(&self) -> Option<String> {
        self.command.identity_opts.identity_name.clone()
    }

    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let authority_node_client = self.authority_client(node.clone()).await?;

        let members = authority_node_client
            .list_members(node.ctx())
            .await?
            .into_iter()
            .filter(|(_, a)| {
                !self.command.enrollers
                    || a.deserialized_key_value_attrs().contains(&format!(
                        "{}={}",
                        OCKAM_ROLE_ATTRIBUTE_KEY, OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE
                    ))
            })
            .map(|(i, a)| MemberOutput::new(i, a))
            .collect::<Vec<_>>();

        let plain = self
            .opts
            .terminal
            .build_list(&members, "No members found on the Authority node")?;
        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(plain)
            .json_obj(&members)?
            .write_line()?;

        Ok(())
    }
}

#[async_trait]
impl Command for ListCommand {
    const NAME: &'static str = "project-member list";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        ListNodeCommand::new(opts.clone(), self.clone())
            .execute(ctx, opts.state)
            .await
    }
}
