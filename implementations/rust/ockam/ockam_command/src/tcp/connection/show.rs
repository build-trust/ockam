use clap::Args;

use colorful::Colorful;
use ockam::Context;
use ockam_api::colors::color_primary;
use ockam_api::nodes::models::transport::TransportStatus;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::{fmt_log, fmt_ok};
use ockam_core::api::Request;

use crate::node::NodeOpts;
use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show a TCP connection
#[derive(Clone, Debug, Args)]
#[command(
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct ShowCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,

    /// TCP connection internal address or socket address
    pub address: String,
}

impl ShowCommand {
    pub fn name(&self) -> String {
        "tcp-connection show".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.node_opts.at_node).await?;
        let transport_status: TransportStatus = node
            .ask(
                ctx,
                Request::get(format!("/node/tcp/connection/{}", &self.address)),
            )
            .await?;

        opts.terminal
            .stdout()
            .plain(
                fmt_ok!("TCP Connection:\n")
                    + &fmt_log!(
                        "  Type: {}\n",
                        color_primary(transport_status.tt.to_string())
                    )
                    + &fmt_log!(
                        "  Mode: {}\n",
                        color_primary(transport_status.tm.to_string())
                    )
                    + &fmt_log!(
                        "  Socket address: {}\n",
                        color_primary(&transport_status.socket_addr)
                    )
                    + &fmt_log!(
                        "  Processor address: {}\n",
                        color_primary(&transport_status.processor_address)
                    )
                    + &fmt_log!(
                        "  Flow Control Id: {}\n",
                        color_primary(transport_status.flow_control_id.to_string())
                    ),
            )
            .write_line()?;

        Ok(())
    }
}
