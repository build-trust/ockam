use clap::Args;
use ockam_api::colors::color_primary;

use crate::node::NodeOpts;
use crate::{docs, CommandGlobalOpts};
use ockam_api::nodes::models::portal::OutletStatusList;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;
use ockam_node::Context;

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");
const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");

/// List all the TCP Outlets at a given node with limited information
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ListCommand {
    #[command(flatten)]
    node_opts: NodeOpts,
}

impl ListCommand {
    pub fn name(&self) -> String {
        "tcp-outlet list".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.node_opts.at_node).await?;

        let spinner = opts.terminal.spinner();
        if let Some(spinner) = &spinner {
            spinner.set_message(format!(
                "Listing TCP Outlets at {}...",
                color_primary(node.node_name())
            ));
        }

        let outlets: OutletStatusList = node.ask(ctx, Request::get("/node/outlet")).await?;
        let outlets = outlets.0;

        if let Some(spinner) = &spinner {
            spinner.finish_and_clear();
        }

        let list = opts.terminal.build_list(
            &outlets,
            &format!(
                "No TCP Outlets found at {}",
                color_primary(node.node_name())
            ),
        )?;

        opts.terminal
            .to_stdout()
            .plain(list)
            .json_obj(outlets)?
            .write_line()?;

        Ok(())
    }
}
