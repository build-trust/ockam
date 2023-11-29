use clap::Args;
use indoc::formatdoc;

use ockam::Context;
use ockam_api::nodes::models::transport::TransportStatus;
use ockam_api::nodes::BackgroundNode;
use ockam_core::api::Request;

use crate::node::NodeOpts;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show a TCP listener
#[derive(Clone, Debug, Args)]
#[command(
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct ShowCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,

    /// TCP listener Worker Address or Tcp Address
    pub address: String,
}

impl ShowCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> miette::Result<()> {
    let node = BackgroundNode::create(&ctx, &opts.state, &cmd.node_opts.at_node).await?;
    let transport_status: TransportStatus = node
        .ask(
            &ctx,
            Request::get(format!("/node/tcp/listener/{}", &cmd.address)),
        )
        .await?;

    let TransportStatus {
        tt,
        tm,
        socket_addr,
        processor_address,
        flow_control_id,
        ..
    } = transport_status;

    let plain = formatdoc! {r#"
        TCP Listener:
          Type: {tt}
          Mode: {tm}
          Socket address: {socket_addr}
          Worker address: {processor_address}
          Flow Control Id: {flow_control_id}
    "#};

    opts.terminal.stdout().plain(plain).write_line()?;

    Ok(())
}
