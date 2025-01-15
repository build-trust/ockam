use clap::Args;
use colorful::Colorful;
use ockam_api::colors::OckamColor;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam_api::nodes::models::transport::TransportStatus;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;
use ockam_node::Context;

use crate::node::NodeOpts;
use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List TCP connections
#[derive(Args, Clone, Debug)]
#[command(
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct ListCommand {
    #[command(flatten)]
    node_opts: NodeOpts,
}

impl ListCommand {
    pub fn name(&self) -> String {
        "tcp-connection list".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.node_opts.at_node).await?;
        let is_finished: Mutex<bool> = Mutex::new(false);

        let get_transports = async {
            let transports: Vec<TransportStatus> =
                node.ask(ctx, Request::get("/node/tcp/connection")).await?;
            *is_finished.lock().await = true;
            Ok(transports)
        };

        let output_messages = vec![format!(
            "Listing TCP Connections on {}...\n",
            node.node_name().color(OckamColor::PrimaryResource.color())
        )];

        let progress_output = opts.terminal.loop_messages(&output_messages, &is_finished);

        let (transports, _) = try_join!(get_transports, progress_output)?;

        let list = opts.terminal.build_list(
            &transports,
            &format!(
                "No TCP Connections found on {}",
                node.node_name().color(OckamColor::PrimaryResource.color())
            ),
        )?;

        opts.terminal.stdout().plain(list).write_line()?;

        Ok(())
    }
}
