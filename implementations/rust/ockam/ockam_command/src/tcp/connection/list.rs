use async_trait::async_trait;
use clap::Args;
use ockam_api::colors::color_primary;

use ockam_api::nodes::models::transport::{TransportMode, TransportStatus, TransportStatusList};
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;
use ockam_node::Context;

use crate::node::NodeOpts;
use crate::{docs, Command, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List TCP Connections
#[derive(Args, Clone, Debug)]
#[command(
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct ListCommand {
    #[command(flatten)]
    node_opts: NodeOpts,
}

#[async_trait]
impl Command for ListCommand {
    const NAME: &'static str = "tcp-connection list";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> crate::Result<()> {
        let node = BackgroundNodeClient::create(ctx, opts.state, &self.node_opts.at_node).await?;
        let node_name = node.node_name();

        let spinner = opts.terminal.spinner();
        if let Some(spinner) = &spinner {
            spinner.set_message(format!(
                "Listing TCP Connections at {}...",
                color_primary(node_name)
            ));
        }

        let transports = Self::get_connection_list(ctx, &node).await?;

        if let Some(spinner) = &spinner {
            spinner.finish_and_clear();
        }

        let list = opts.terminal.build_list(
            &transports,
            &format!("No TCP Connections found on {}", color_primary(node_name)),
        )?;

        opts.terminal
            .to_stdout()
            .plain(list)
            .json_obj(transports)?
            .write_line()?;

        Ok(())
    }
}

impl ListCommand {
    pub(super) async fn get_connection_list(
        ctx: &Context,
        node: &BackgroundNodeClient,
    ) -> crate::Result<Vec<TransportStatus>> {
        let transports: TransportStatusList =
            node.ask(ctx, Request::get("/node/tcp/connection")).await?;
        let mut transports = transports.0;
        // Filter out the last Incoming connection, which is the one created by the current command execution
        let self_transport = transports
            .iter()
            .rev()
            .position(|t| t.tm == TransportMode::Incoming);
        if let Some(rev_position) = self_transport {
            let position = transports.len() - 1 - rev_position;
            transports.remove(position);
        }
        Ok(transports)
    }
}
