use clap::Args;
use colorful::Colorful;
use ockam_api::colors::color_primary;
use ockam_api::fmt_info;
use tokio::sync::Mutex;
use tokio::try_join;

use crate::node::NodeOpts;
use crate::{docs, CommandGlobalOpts};
use ockam_api::nodes::models::portal::OutletStatus;
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

        let is_finished: Mutex<bool> = Mutex::new(false);

        let send_req = async {
            let res: Vec<OutletStatus> = node.ask(ctx, Request::get("/node/outlet")).await?;
            *is_finished.lock().await = true;
            Ok(res)
        };

        let output_messages = vec![format!(
            "Listing TCP Outlets on node {}...\n",
            color_primary(node.node_name())
        )];

        let progress_output = opts.terminal.loop_messages(&output_messages, &is_finished);

        let (outlets, _) = try_join!(send_req, progress_output)?;

        let list: String = {
            let empty_message = fmt_info!(
                "No TCP Outlets found on node {}",
                color_primary(node.node_name())
            );
            match outlets.is_empty() {
                true => empty_message,
                false => opts.terminal.build_list(&outlets, &empty_message)?,
            }
        };

        let json: Vec<_> = outlets
            .iter()
            .map(|outlet| {
                Ok(serde_json::json!({
                    "from": outlet.worker_route()?,
                    "to": outlet.to,
                }))
            })
            .flat_map(|res: Result<_, ockam_core::Error>| res.ok())
            .collect();

        opts.terminal
            .stdout()
            .plain(list)
            .json(serde_json::json!(json))
            .write_line()?;

        Ok(())
    }
}
