use async_trait::async_trait;
use clap::Args;
use std::sync::Arc;

use ockam::Context;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::space::Spaces;

use crate::node_command::InMemoryNodeCommand;
use crate::shared_args::IdentityOpts;
use crate::{docs, Command, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List spaces
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ListCommand {
    #[command(flatten)]
    pub identity_opts: IdentityOpts,
}

#[derive(Clone)]
struct ListNodeCommand {
    opts: CommandGlobalOpts,
}
impl ListNodeCommand {
    pub fn new(opts: CommandGlobalOpts) -> Self {
        Self { opts }
    }
}
#[async_trait]
impl InMemoryNodeCommand for ListNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let spaces = {
            let pb = self.opts.terminal.spinner();
            if let Some(pb) = pb.as_ref() {
                pb.set_message("Listing spaces...");
            }
            node.get_spaces().await?
        };

        let plain = self.opts.terminal.build_list(
            &spaces,
            "No spaces found. Run 'ockam enroll' to get a space and a project",
        )?;

        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(plain)
            .json_obj(&spaces)?
            .write_line()?;
        Ok(())
    }
}

#[async_trait]
impl Command for ListCommand {
    const NAME: &'static str = "space list";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        ListNodeCommand::new(opts.clone())
            .execute(ctx, opts.state)
            .await
    }
}
