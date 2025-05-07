use crate::cluster::utils::get_cluster;
use crate::node_command::InMemoryNodeCommand;
use crate::{docs, Command, CommandGlobalOpts, Result};
use async_trait::async_trait;
use clap::Args;
use colorful::Colorful;
use ockam_api::colors::color_primary;
use ockam_api::fmt_ok;
use ockam_api::nodes::InMemoryNode;
use ockam_node::Context;
use std::sync::Arc;

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Shows your Cluster ID
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {}

#[derive(Clone)]
struct DeployNodeCommand {
    opts: CommandGlobalOpts,
}

#[async_trait]
impl InMemoryNodeCommand for DeployNodeCommand {
    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let ctx = node.ctx();
        let cluster = get_cluster(ctx, &node).await?;
        self.opts
            .terminal
            .clone()
            .to_stdout()
            .plain(fmt_ok!("Your cluster is {}", color_primary(&cluster)))
            .machine(&cluster)
            .json_obj(serde_json::json!({"cluster": cluster}))?
            .write_line()?;
        Ok(())
    }
}

#[async_trait]
impl Command for ShowCommand {
    const NAME: &'static str = "cluster show";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let command = DeployNodeCommand { opts: opts.clone() };
        command.execute(ctx, opts.state.clone()).await?;
        Ok(())
    }
}
