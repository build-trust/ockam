use clap::Args;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::nodes::models::forwarder::ForwarderInfo;
use ockam_api::nodes::BackgroundNode;
use ockam_core::api::Request;

use crate::node::get_node_name;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show a Relay by its alias
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = false,
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    /// Name assigned to relay that will be shown (prefixed with forward_to_<name>)
    #[arg(display_order = 900, required = true)]
    remote_address: String,

    /// Node which relay belongs to
    #[arg(display_order = 901, global = true, long, value_name = "NODE")]
    pub at: Option<String>,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> miette::Result<()> {
    let at = get_node_name(&opts.state, &cmd.at);
    let node_name = extract_address_value(&at)?;
    let remote_address = &cmd.remote_address;
    let node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;
    let relay_info: ForwarderInfo = node
        .ask(
            &ctx,
            Request::get(format!("/node/forwarder/{remote_address}")),
        )
        .await?;

    println!("Relay:");
    println!("  Relay Route: {}", relay_info.forwarding_route());
    println!(
        "  Remote Address: {}",
        relay_info.remote_address_ma().into_diagnostic()?
    );
    println!(
        "  Worker Address: {}",
        relay_info.worker_address_ma().into_diagnostic()?
    );

    Ok(())
}
