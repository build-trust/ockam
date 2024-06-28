use clap::Args;

use ockam_api::nodes::models::portal::InletStatus;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;
use ockam_node::Context;

use crate::node::NodeOpts;
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List TCP Inlets on the default node
#[derive(Args, Clone, Debug)]
#[command(
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct ListCommand {
    #[command(flatten)]
    node: NodeOpts,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "tcp-inlet list".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.node.at_node).await?;
        let inlets: Vec<InletStatus> = {
            let pb = opts.terminal.progress_bar();
            if let Some(pb) = pb.as_ref() {
                pb.set_message(format!("Listing TCP Inlets on {}...", node.node_name()));
            }
            node.ask(ctx, Request::get("/node/inlet")).await?
        };

        let plain = opts.terminal.build_list(
            &inlets,
            &format!("No TCP Inlets found on {}", node.node_name()),
        )?;
        opts.terminal
            .stdout()
            .plain(plain)
            .json_obj(&inlets)?
            .write_line()?;

        Ok(())
    }
}
