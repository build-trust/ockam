use crate::node_command::InMemoryNodeCommand;
use crate::shared_args::IdentityOpts;
use crate::{Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use ockam::Context;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::space::Spaces;
use std::sync::Arc;

/// List the Admins of a Space
#[derive(Clone, Debug, Args)]
#[command()]
pub struct ListCommand {
    /// Name of the Space
    name: Option<String>,

    #[command(flatten)]
    identity_opts: IdentityOpts,
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
    fn identity_name(&self) -> Option<String> {
        self.command.identity_opts.identity_name.clone()
    }

    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let space = self
            .opts
            .state
            .get_space_by_name_or_default(&self.command.name)
            .await?;
        let admins = node.list_space_admins(&space.space_id()).await?;

        let list = &self.opts.terminal.build_list(&admins, "No admins found")?;
        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(list)
            .json_obj(admins)?
            .write_line()?;
        Ok(())
    }
}

#[async_trait]
impl Command for ListCommand {
    const NAME: &'static str = "space-admin list";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        ListNodeCommand::new(opts.clone(), self.clone())
            .execute(ctx, opts.state)
            .await
    }
}
