use clap::Args;

use ockam::Context;
use ockam_api::nodes::models::secure_channel::ShowSecureChannelResponse;
use ockam_api::nodes::BackgroundNode;
use ockam_core::Address;

use crate::node::get_node_name;
use crate::util::parse_node_name;
use crate::{
    docs,
    util::{api, node_rpc},
    CommandGlobalOpts,
};

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show Secure Channels
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = true,
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct ShowCommand {
    /// Node at which the secure channel was initiated
    #[arg(value_name = "NODE_NAME", long, display_order = 800)]
    at: Option<String>,

    /// Channel address
    #[arg(display_order = 800)]
    address: Address,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, ShowCommand)) -> miette::Result<()> {
    let at = get_node_name(&opts.state, &cmd.at);
    let node_name = parse_node_name(&at)?;
    let address = &cmd.address;

    let node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;
    let response: ShowSecureChannelResponse =
        node.ask(&ctx, api::show_secure_channel(address)).await?;
    opts.println(&response)?;
    Ok(())
}
