use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};

use ockam::Context;
use ockam_api::fmt_ok;
use ockam_api::nodes::models::transport::TransportStatus;
use ockam_api::nodes::{models, BackgroundNodeClient};
use ockam_core::api::Request;

use crate::{docs, node::NodeOpts, CommandGlobalOpts};

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a TCP listener
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// TCP Listener internal address
    pub address: String,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn name(&self) -> String {
        "tcp-listener delete".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.node_opts.at_node).await?;

        // Check if there an TCP listener with the provided address exists
        let address = self.address.clone();
        node.ask_and_get_reply::<_, TransportStatus>(
            ctx,
            Request::get(format!("/node/tcp/listener/{address}")),
        )
        .await?
        .found()
        .into_diagnostic()?
        .ok_or(miette!(
            "TCP listener with address {address} was not found on Node {}",
            node.node_name()
        ))?;

        // Proceed with the deletion
        if opts.terminal.confirmed_with_flag_or_prompt(
            self.yes,
            "Are you sure you want to delete this TCP listener?",
        )? {
            let req = Request::delete("/node/tcp/listener")
                .body(models::transport::DeleteTransport::new(address.clone()));
            node.tell(ctx, req).await?;

            opts.terminal
                .stdout()
                .plain(fmt_ok!(
                    "TCP listener with address {address} on Node {} has been deleted",
                    node.node_name()
                ))
                .json(serde_json::json!({"node": node.node_name() }))
                .write_line()
                .unwrap();
        }
        Ok(())
    }
}
