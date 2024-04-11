use clap::Args;
use colorful::Colorful;
use ockam_api::colors::OckamColor;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam_api::nodes::models::transport::TransportList;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;
use ockam_node::Context;

use crate::node::NodeOpts;
use crate::{docs, CommandGlobalOpts};

use crate::util::async_cmd;

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
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "tcp-connection list".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.node_opts.at_node).await?;
        let is_finished: Mutex<bool> = Mutex::new(false);

        let get_transports = async {
            let transports: TransportList =
                node.ask(ctx, Request::get("/node/tcp/connection")).await?;
            *is_finished.lock().await = true;
            Ok(transports)
        };

        let output_messages = vec![format!(
            "Listing TCP Connections on {}...\n",
            node.node_name().color(OckamColor::PrimaryResource.color())
        )];

        let progress_output = opts
            .terminal
            .progress_output(&output_messages, &is_finished);

        let (transports, _) = try_join!(get_transports, progress_output)?;

        let list = opts.terminal.build_list(
            &transports.list,
            &format!("TCP Connections on {}", node.node_name()),
            &format!(
                "No TCP Connections found on {}",
                node.node_name().color(OckamColor::PrimaryResource.color())
            ),
        )?;

        opts.terminal.stdout().plain(list).write_line()?;

        Ok(())
    }
}
