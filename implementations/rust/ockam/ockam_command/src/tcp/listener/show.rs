use clap::Args;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_api::nodes::models::transport::TransportStatus;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;

use crate::node::NodeOpts;
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

    /// TCP listener internal address or socket address
    pub address: String,
}

impl ShowCommand {
    pub fn name(&self) -> String {
        "tcp-listener show".into()
    }

    pub async fn run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.node_opts.at_node).await?;
        let transport_status: TransportStatus = node
            .ask(
                ctx,
                Request::get(format!("/node/tcp/listener/{}", &self.address)),
            )
            .await?;
        opts.terminal
            .stdout()
            .plain(&transport_status)
            .json(serde_json::to_string(&transport_status).into_diagnostic()?)
            .write_line()?;
        Ok(())
    }
}
