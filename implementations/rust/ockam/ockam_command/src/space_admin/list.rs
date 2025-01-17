use crate::shared_args::IdentityOpts;
use crate::{Command, CommandGlobalOpts};
use async_trait::async_trait;
use clap::Args;
use ockam::Context;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::space::Spaces;

/// List the Admins of a Space
#[derive(Clone, Debug, Args)]
#[command()]
pub struct ListCommand {
    /// Name of the Space
    name: Option<String>,

    #[command(flatten)]
    identity_opts: IdentityOpts,
}

#[async_trait]
impl Command for ListCommand {
    const NAME: &'static str = "space-admin list";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        let space = opts.state.get_space_by_name_or_default(&self.name).await?;
        let node =
            InMemoryNode::start_with_identity(ctx, &opts.state, self.identity_opts.identity_name)
                .await?;
        let admins = node.list_space_admins(ctx, &space.space_id()).await?;

        let list = &opts.terminal.build_list(&admins, "No admins found")?;
        opts.terminal
            .stdout()
            .plain(list)
            .json_obj(admins)?
            .write_line()?;
        Ok(())
    }
}
