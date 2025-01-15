use clap::Args;

use ockam::Context;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::Address;

use crate::node::NodeOpts;
use crate::util::api;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show Secure Channel Listener
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct ShowCommand {
    /// Address of the channel listener
    address: Address,

    #[command(flatten)]
    node_opts: NodeOpts,
}

impl ShowCommand {
    pub fn name(&self) -> String {
        "show secure channel listener".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.node_opts.at_node).await?;
        let address = &self.address;
        let req = api::show_secure_channel_listener(address);
        node.tell(ctx, req).await?;
        opts.terminal
            .stdout()
            .plain(format!("/service/{}", self.address.address()))
            .write_line()?;
        Ok(())
    }
}
