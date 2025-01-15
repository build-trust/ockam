use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_api::fmt_ok;
use ockam_api::nodes::models::secure_channel::DeleteSecureChannelListenerResponse;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::Address;

use crate::node::NodeOpts;
use crate::util::api;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/delete/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete Secure Channel Listeners
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct DeleteCommand {
    /// Address at which the channel listener to be deleted is running
    address: Address,

    #[command(flatten)]
    node_opts: NodeOpts,
}

impl DeleteCommand {
    pub fn name(&self) -> String {
        "delete secure channel listener".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.node_opts.at_node).await?;
        let req = api::delete_secure_channel_listener(&self.address);
        let response: DeleteSecureChannelListenerResponse = node.ask(ctx, req).await?;
        let addr = response.addr;
        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "Deleted secure-channel listener with address '{addr}' on node '{}'",
                node.node_name()
            ))
            .machine(addr)
            .write_line()?;
        Ok(())
    }
}
