use clap::Args;

use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::colors::color_primary;
use ockam_api::nodes::models::relay::RelayInfo;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;

use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List Relays
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = false,
    before_help = docs::before_help(PREVIEW_TAG),
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ListCommand {
    /// Get the list of Relays at the given node
    #[arg(global = true, long, value_name = "NODE", value_parser = extract_address_value)]
    pub to: Option<String>,
}

impl ListCommand {
    pub fn name(&self) -> String {
        "relay list".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.to).await?;
        let relays: Vec<RelayInfo> = {
            let pb = opts.terminal.spinner();
            if let Some(pb) = pb {
                pb.set_message(format!(
                    "Listing Relays on {}...\n",
                    color_primary(node.node_name())
                ));
            }
            node.ask(ctx, Request::get("/node/relay")).await?
        };
        let plain = opts.terminal.build_list(
            &relays,
            &format!("No Relays found on node {}", node.node_name()),
        )?;
        opts.terminal
            .stdout()
            .plain(plain)
            .json_obj(relays)?
            .write_line()?;
        Ok(())
    }
}
